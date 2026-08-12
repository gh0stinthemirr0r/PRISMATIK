//! The tool surface an agent may reach, and the registry that governs it.
//!
//! Two design decisions are load-bearing.
//!
//! **Every tool here is read-only, and that is enforced by construction.** The
//! registry is built from a fixed list and each entry declares its
//! [`ToolClass`]; `dispatch` refuses anything that is not
//! [`ToolClass::ReadOnly`]. An agent cannot place an order, move money, or
//! change configuration, because no such tool exists in this table — not
//! because a prompt asks it not to. Order submission stays behind the trader
//! loop's risk gate and the live-execution arming, where it is auditable.
//!
//! **The protocol is text, not provider-native function calling.** PRISMATIK
//! talks to eleven provider families including loopback local models, whose
//! tool-calling schemas differ or are absent entirely. A small JSON protocol
//! the model emits as text works identically everywhere, degrades to a plain
//! answer when a model ignores it, and is trivially auditable because the exact
//! bytes are logged. The cost is a little parsing; the benefit is that a local
//! Llama and a frontier model run the same agent loop.

use prismatik_ai_tools::tool::{ControlledTool, ToolClass};
use serde::{Deserialize, Serialize};

/// A tool the agent can call, described well enough for a model to use it.
pub(crate) struct AgentTool {
    pub(crate) name: &'static str,
    pub(crate) class: ToolClass,
    pub(crate) description: &'static str,
    /// Argument shape, described in prose because it goes into a prompt.
    pub(crate) arguments: &'static str,
}

impl ControlledTool for AgentTool {
    fn id(&self) -> &str {
        self.name
    }
    fn class(&self) -> ToolClass {
        self.class
    }
}

/// The complete read-only tool surface.
pub(crate) const TOOLS: &[AgentTool] = &[
    AgentTool {
        name: "list_tracked",
        class: ToolClass::ReadOnly,
        description: "List the instruments the desk is tracking, with kind and venue.",
        arguments: "{} (no arguments)",
    },
    AgentTool {
        name: "get_quotes",
        class: ToolClass::ReadOnly,
        description: "Current quotes for tracked instruments, with provider and observation time.",
        arguments: "{} (no arguments)",
    },
    AgentTool {
        name: "analyze",
        class: ToolClass::ReadOnly,
        description:
            "Deterministic analytics for one tracked symbol: volatility regime, how long it has \
             held, conditional probability, the unconditional base rate, the edge between them, \
             and measured forecaster skill.",
        arguments: "{\"symbol\": \"AAPL\"}",
    },
    AgentTool {
        name: "calibration",
        class: ToolClass::ReadOnly,
        description:
            "Scored standing of every forecasting cohort: Brier, ECE, and skill against the base \
             rate. Use this to judge whether a forecaster is worth believing.",
        arguments: "{} (no arguments)",
    },
    AgentTool {
        name: "yield_curve",
        class: ToolClass::ReadOnly,
        description: "Current US treasury term structure and curve shape from FRED.",
        arguments: "{} (no arguments)",
    },
    AgentTool {
        name: "portfolio",
        class: ToolClass::ReadOnly,
        description: "Paper positions with marks, exposure and unrealised P&L.",
        arguments: "{} (no arguments)",
    },
];

/// A tool call parsed out of a model turn.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct ToolCall {
    pub(crate) tool: String,
    #[serde(default)]
    pub(crate) symbol: Option<String>,
}

/// One executed tool call, kept for the audit trail.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolTrace {
    pub(crate) tool: String,
    pub(crate) argument: Option<String>,
    pub(crate) ok: bool,
    /// Result as handed back to the model, truncated for the transcript.
    pub(crate) result: String,
}

/// Render the tool table into the instruction block.
pub(crate) fn describe() -> String {
    let mut out = String::from(
        "You may call tools to gather evidence before answering. To call one, reply with a \
         single line containing ONLY a JSON object:\n\
         {\"tool\": \"<name>\", \"symbol\": \"<optional symbol>\"}\n\
         You will then receive the result and may call another tool or answer. \
         When you are ready to answer, reply normally with no JSON object.\n\n\
         Available tools (all read-only — none can place an order or change any setting):\n",
    );
    for tool in TOOLS {
        out.push_str(&format!(
            "- {}: {} Arguments: {}\n",
            tool.name, tool.description, tool.arguments
        ));
    }
    out
}

/// Extract a tool call from a model turn.
///
/// Deliberately strict: the JSON object must be the entire trimmed turn, or a
/// fenced code block containing only that object. A model that writes prose
/// *about* calling a tool is answering, not calling — treating a mention as an
/// invocation would let evidence text steer execution.
pub(crate) fn parse_call(turn: &str) -> Option<ToolCall> {
    let trimmed = turn.trim();
    let candidate = if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest.strip_prefix("json").unwrap_or(rest);
        body.trim().strip_suffix("```")?.trim()
    } else {
        trimmed
    };
    if !candidate.starts_with('{') || !candidate.ends_with('}') {
        return None;
    }
    let call: ToolCall = serde_json::from_str(candidate).ok()?;
    // Unknown tool names are not calls; the loop will surface the raw turn.
    TOOLS
        .iter()
        .any(|tool| tool.name == call.tool)
        .then_some(call)
}

/// Execute a tool call.
///
/// Returns the result as text for the model. Errors are returned as text too,
/// not propagated: a failing tool is information the agent should reason about
/// ("the provider is down") rather than an abort.
pub(crate) async fn dispatch(call: &ToolCall) -> ToolTrace {
    let Some(tool) = TOOLS.iter().find(|tool| tool.name == call.tool) else {
        return ToolTrace {
            tool: call.tool.clone(),
            argument: None,
            ok: false,
            result: "unknown tool".to_owned(),
        };
    };
    // Belt and braces: the table is read-only by construction, but if a
    // risk-increasing tool is ever added it must not become reachable by an
    // agent merely because it was registered.
    if tool.class != ToolClass::ReadOnly {
        return ToolTrace {
            tool: call.tool.clone(),
            argument: None,
            ok: false,
            result: "tool is not read-only and is not callable by an agent".to_owned(),
        };
    }

    let result = match call.tool.as_str() {
        "list_tracked" => list_tracked(),
        "get_quotes" => quotes().await,
        "analyze" => match call.symbol.as_deref() {
            Some(symbol) => analyze(symbol).await,
            None => Err("analyze requires a symbol".to_owned()),
        },
        "calibration" => calibration(),
        "yield_curve" => yield_curve().await,
        "portfolio" => portfolio().await,
        other => Err(format!("unknown tool {other}")),
    };

    match result {
        Ok(text) => ToolTrace {
            tool: call.tool.clone(),
            argument: call.symbol.clone(),
            ok: true,
            result: truncate(&text, 4_000),
        },
        Err(error) => ToolTrace {
            tool: call.tool.clone(),
            argument: call.symbol.clone(),
            ok: false,
            result: truncate(&error, 1_000),
        },
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated]", &text[..end])
}

fn list_tracked() -> Result<String, String> {
    let app = crate::app_handle()?;
    let rows = crate::tracking::read_tracked(&app)?;
    if rows.is_empty() {
        return Ok("No instruments are tracked.".to_owned());
    }
    Ok(rows
        .iter()
        .map(|row| format!("{} ({:?}, {})", row.symbol, row.kind, row.market))
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn quotes() -> Result<String, String> {
    let snapshot = crate::terminal_feed::get_terminal_feed(crate::app_handle()?).await?;
    if snapshot.quotes.is_empty() {
        return Ok(format!("No quotes available: {}", snapshot.message));
    }
    Ok(snapshot
        .quotes
        .iter()
        .map(|q| {
            format!(
                "{} {:.6} ({} at {})",
                q.symbol, q.price, q.provider, q.observed_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn analyze(symbol: &str) -> Result<String, String> {
    crate::quant_context::for_subject(symbol)
        .await
        .ok_or_else(|| format!("{symbol} is not tracked or has no classifiable history"))
}

fn calibration() -> Result<String, String> {
    let rows = crate::forecast_candidates::forecast_calibration_health()?;
    if rows.is_empty() {
        return Ok("No scored cohorts yet.".to_owned());
    }
    Ok(serde_json::to_string_pretty(&rows).unwrap_or_else(|error| error.to_string()))
}

async fn yield_curve() -> Result<String, String> {
    let curve = crate::fixed_income::get_yield_curve().await?;
    Ok(serde_json::to_string_pretty(&curve).unwrap_or_else(|error| error.to_string()))
}

async fn portfolio() -> Result<String, String> {
    let oms = crate::paper_oms::get_paper_oms().await?;
    Ok(serde_json::to_string_pretty(&oms).unwrap_or_else(|error| error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_tool_is_read_only() {
        // The safety property the whole harness rests on.
        assert!(TOOLS.iter().all(|tool| tool.class == ToolClass::ReadOnly));
    }

    #[test]
    fn tool_names_are_unique() {
        let mut names: Vec<&str> = TOOLS.iter().map(|tool| tool.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn a_bare_json_object_parses_as_a_call() {
        let call = parse_call(r#"{"tool": "analyze", "symbol": "AAPL"}"#).unwrap();
        assert_eq!(call.tool, "analyze");
        assert_eq!(call.symbol.as_deref(), Some("AAPL"));
    }

    #[test]
    fn a_fenced_json_object_parses_as_a_call() {
        let call = parse_call("```json\n{\"tool\": \"get_quotes\"}\n```").unwrap();
        assert_eq!(call.tool, "get_quotes");
    }

    #[test]
    fn prose_mentioning_a_tool_is_not_a_call() {
        // The important negative case: text that merely talks about calling a
        // tool must be treated as an answer, or evidence text could steer
        // execution simply by describing it.
        assert!(parse_call("I would call {\"tool\": \"analyze\"} next.").is_none());
        assert!(parse_call("Let me use the analyze tool on AAPL.").is_none());
        assert!(parse_call("The answer is that analyze returns a regime.").is_none());
    }

    #[test]
    fn an_unknown_tool_name_is_not_a_call() {
        assert!(parse_call(r#"{"tool": "submit_order", "symbol": "AAPL"}"#).is_none());
        assert!(parse_call(r#"{"tool": "rm -rf"}"#).is_none());
    }

    #[test]
    fn malformed_json_is_not_a_call() {
        assert!(parse_call("{tool: analyze}").is_none());
        assert!(parse_call("{").is_none());
        assert!(parse_call("").is_none());
    }

    #[test]
    fn the_description_lists_every_tool() {
        let described = describe();
        for tool in TOOLS {
            assert!(described.contains(tool.name), "{} missing", tool.name);
        }
        assert!(described.contains("read-only"));
    }

    #[test]
    fn truncation_never_splits_a_character() {
        let text = "é".repeat(3_000);
        let cut = truncate(&text, 1_000);
        assert!(cut.ends_with("… [truncated]"));
        // Would panic on a bad boundary.
        assert!(cut.chars().count() > 0);
    }
}
