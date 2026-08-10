//! Agent council — multi-agent debate loop for autonomous trading intelligence.
//!
//! Directive #4 and #15. This is the "agent/bot" surface: a structured
//! multi-agent debate that mirrors a real trading desk. Roles:
//!
//! 1. **Technical analyst** — reads price action, momentum, regime.
//! 2. **Fundamental analyst** — reads macro/filings evidence.
//! 3. **Bull researcher** — argues the long case from the evidence.
//! 4. **Bear researcher** — argues the short case (adversarial falsification).
//! 5. **Risk manager** — evaluates position sizing, drawdown, kill-switch.
//! 6. **Portfolio manager** — synthesizes into a final decision.
//!
//! Each role is a single LLM call with a role-specific system prompt over the
//! same real governed evidence packet. The architecture's AnalystScope
//! correction (Part II §14.3) applies: agents see the *same* facts but reason
//! independently, and the bear agent is explicitly adversarial.
//!
//! The output is a structured `AgentCouncilResult` with a recommendation that
//! is **never auto-executed** — it is surfaced to the operator, who decides.
//! If the operator approves, it routes through the paper OMS + risk gate
//! (1%-per-trade + 10% drawdown kill switch), never around them.

use prismatik_application::{invoke_model_http, ModelHttpRequest, ModelProvider};
use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

use crate::model_integrations::{session, ModelSession};

/// A single agent's contribution to the debate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurn {
    /// Role name (e.g. "technical_analyst").
    pub role: String,
    /// Display label (e.g. "Technical Analyst").
    pub label: String,
    /// The agent's analysis text.
    pub analysis: String,
    /// Stance this agent took.
    pub stance: Stance,
    /// Conviction 0.0..1.0 self-reported.
    pub conviction: f64,
    /// Model that produced this.
    pub model: String,
    /// Input/output token counts if available.
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Trading stance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// Bullish.
    Long,
    /// Bearish.
    Short,
    /// No edge / wait.
    Neutral,
}

/// Final synthesized recommendation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecommendation {
    /// Net stance after the PM synthesizes the debate.
    pub stance: Stance,
    /// Conviction 0.0..1.0.
    pub conviction: f64,
    /// Recommended action narrative.
    pub rationale: String,
    /// Suggested entry (if any).
    pub suggested_entry: Option<f64>,
    /// Suggested stop (if any) — required for any risk-sized position.
    pub suggested_stop: Option<f64>,
    /// Suggested target.
    pub suggested_target: Option<f64>,
    /// Whether the risk manager approved.
    pub risk_approved: bool,
    /// Risk manager's caveats.
    pub risk_caveats: Vec<String>,
    /// Configurations-searched disclosure (honesty: how many agents ran).
    pub agents_consulted: usize,
}

/// Full council debate result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCouncilResult {
    pub subject: String,
    pub question: String,
    pub turns: Vec<AgentTurn>,
    pub recommendation: AgentRecommendation,
    pub evidence_count: usize,
    pub evidence_ids: Vec<String>,
    pub generated_at: String,
    pub execution_eligible: bool,
    pub message: String,
}

/// Request to run a council debate.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CouncilRequest {
    /// The subject instrument (e.g. "BTC", "AAPL", "ES").
    pub subject: String,
    /// The question to debate.
    pub question: String,
    /// Model to use for all agents (e.g. "gpt-4o", "claude-sonnet-4").
    pub model: String,
    /// Max output tokens per agent.
    pub max_output_tokens: Option<u32>,
}

const ROLE_TECHNICAL: &str = "technical_analyst";
const ROLE_FUNDAMENTAL: &str = "fundamental_analyst";
const ROLE_BULL: &str = "bull_researcher";
const ROLE_BEAR: &str = "bear_researcher";
const ROLE_RISK: &str = "risk_manager";
const ROLE_PM: &str = "portfolio_manager";

fn system_prompt(role: &str, subject: &str) -> String {
    let base = format!(
        "You are a {role} on a systematic trading desk analyzing {subject}. \
        You receive real governed market evidence. Reason rigorously. \
        Be honest about uncertainty — never claim certainty you do not have. \
        Respond in under 300 words. End with a line: \
        'STANCE: long|short|neutral' and 'CONVICTION: 0.0-1.0'."
    );
    match role {
        ROLE_TECHNICAL => format!("{base} Focus on price action, trend, momentum, volatility regime, and support/resistance."),
        ROLE_FUNDAMENTAL => format!("{base} Focus on macro conditions, filings, and fundamental drivers visible in the evidence."),
        ROLE_BULL => format!("{base} Your ONLY job is to build the strongest possible long case. Be persuasive but evidence-based."),
        ROLE_BEAR => format!("{base} Your ONLY job is to build the strongest possible short case and falsify the bull thesis. Default to skeptical. This is adversarial falsification — find the weakness."),
        ROLE_RISK => format!("{base} Evaluate position sizing, drawdown risk, and whether a trade here respects a 1%-of-equity risk budget with a defined stop. Flag circuit-breaker concerns."),
        ROLE_PM => format!("{base} Synthesize the analysts' and researchers' debate into ONE decision. Weigh the bull vs bear cases. Produce a final stance, conviction, entry/stop/target if actionable, and a one-paragraph rationale."),
        _ => base,
    }
}

fn extract_stance(text: &str) -> (Stance, f64) {
    let lower = text.to_ascii_lowercase();
    let stance = if lower.contains("stance: long") || lower.contains("stance:long") {
        Stance::Long
    } else if lower.contains("stance: short") || lower.contains("stance:short") {
        Stance::Short
    } else {
        Stance::Neutral
    };
    let conviction = lower
        .find("conviction:")
        .and_then(|i| lower[i..].split(':').nth(1))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.trim_end_matches(',').parse::<f64>().ok())
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(0.5);
    (stance, conviction)
}

/// Arguments bundle for a single agent turn (avoids >8-arg clippy lint).
struct AgentTurnArgs<'a> {
    session: &'a ModelSession,
    model: &'a str,
    role: &'a str,
    label: &'a str,
    subject: &'a str,
    question: &'a str,
    evidence_block: &'a str,
    prior_debate: &'a str,
    max_tokens: u32,
}

async fn run_agent_turn(args: AgentTurnArgs<'_>) -> Result<AgentTurn, String> {
    let user_prompt = format!(
        "QUESTION: {}\n\nEVIDENCE:\n{}\n\nPRIOR DEBATE:\n{}\n\n\
        Provide your {} analysis now.",
        args.question, args.evidence_block, args.prior_debate, args.label
    );
    let (provider_id, text, input_tokens, output_tokens) = invoke_llm(
        args.session,
        args.model,
        &system_prompt(args.role, args.subject),
        &user_prompt,
        args.max_tokens,
    )
    .await?;
    let (stance, conviction) = extract_stance(&text);
    Ok(AgentTurn {
        role: args.role.to_string(),
        label: args.label.to_string(),
        analysis: text,
        stance,
        conviction,
        model: format!("{provider_id}/{}", args.model),
        input_tokens,
        output_tokens,
    })
}

async fn invoke_llm(
    session: &ModelSession,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: u32,
) -> Result<(String, String, Option<u64>, Option<u64>), String> {
    let (provider, api_key, local_endpoint): (ModelProvider, Option<String>, Option<String>) =
        match session {
            ModelSession::OpenAi { api_key } => {
                (ModelProvider::OpenAi, Some(api_key.clone()), None)
            }
            ModelSession::Anthropic { api_key } => {
                (ModelProvider::Anthropic, Some(api_key.clone()), None)
            }
            ModelSession::Google { api_key } => {
                (ModelProvider::Google, Some(api_key.clone()), None)
            }
            ModelSession::OpenAiCompatible { base_url, api_key } => {
                // xAI/DeepSeek/Groq/Cohere/OpenRouter/Together/Fireworks all
                // speak the OpenAI-compatible protocol. Route through the
                // local-compatible adapter with the cloud base URL.
                (ModelProvider::LocalCompatible, Some(api_key.clone()), Some(base_url.clone()))
            }
            ModelSession::Local { endpoint, api_key } => {
                (ModelProvider::LocalCompatible, api_key.clone(), Some(endpoint.clone()))
            }
        };
    let provider_label = format!("{provider:?}").to_ascii_lowercase();
    let request = ModelHttpRequest {
        provider,
        model: model.to_string(),
        api_key,
        local_endpoint,
        instruction: system_prompt.to_string(),
        evidence: user_prompt.to_string(),
        max_output_tokens: max_tokens,
    };
    let response = invoke_model_http(&request)
        .await
        .map_err(|e| format!("model invocation failed: {e}"))?;
    Ok((
        provider_label,
        response.text,
        response.input_tokens,
        response.output_tokens,
    ))
}

/// Run a full agent council debate. This is the autonomous intelligence
/// surface — it produces a structured, evidence-cited recommendation but
/// NEVER auto-executes. The operator decides; the risk gate enforces.
#[tauri::command]
pub(crate) async fn run_agent_council(
    req: CouncilRequest,
) -> Result<AgentCouncilResult, String> {
    if req.subject.trim().is_empty() || req.question.trim().is_empty() {
        return Err("subject and question are required".into());
    }
    if req.model.trim().is_empty() {
        return Err("model is required".into());
    }
    // Use whichever provider has an active session. Try them in priority order.
    let active_session = [
        "openai",
        "anthropic",
        "google",
        "xai",
        "deepseek",
        "groq",
        "cohere",
        "openrouter",
        "together",
        "fireworks",
        "local",
    ]
    .into_iter()
    .find_map(|p| session(p).map(|s| (p, s)))
    .ok_or_else(|| "no active model provider session — connect a provider first".to_string())?;
    let max_tokens = req.max_output_tokens.unwrap_or(500).clamp(100, 2000);

    // Gather real governed evidence.
    let snapshot = crate::terminal_feed::get_terminal_feed().await?;
    let durable_evidence = crate::evidence_store::model_evidence()?;
    let subject_lower = req.subject.to_ascii_lowercase();
    let relevant_quotes: Vec<_> = snapshot
        .quotes
        .iter()
        .filter(|q| q.symbol.to_ascii_lowercase().contains(&subject_lower))
        .collect();
    let evidence_block = if relevant_quotes.is_empty() && durable_evidence.is_empty() {
        return Err("no real governed evidence available for this subject; the agent council refuses to reason without evidence".into());
    } else if relevant_quotes.is_empty() {
        format!("Durable observations (macro/filings):\n{}", durable_evidence_summary(&durable_evidence))
    } else {
        let mut block = String::new();
        for q in relevant_quotes.iter().take(10) {
            block.push_str(&format!(
                "- {} {} @ {:.6} (provider: {}, observed: {})\n",
                q.symbol, q.price, q.price, q.provider, q.observed_at
            ));
        }
        if !durable_evidence.is_empty() {
            block.push_str(&format!("\n{}", durable_evidence_summary(&durable_evidence)));
        }
        block
    };
    let evidence_ids: Vec<String> = relevant_quotes
        .iter()
        .map(|q| format!("market:{}:{}:{}", q.provider, q.symbol, q.observed_at))
        .collect();

    let session = active_session.1;
    let subject = &req.subject;
    let question = &req.question;

    // Phase 1: parallel analysts (technical + fundamental).
    let (technical, fundamental) = tokio::join!(
        run_agent_turn(AgentTurnArgs {
            session: &session,
            model: &req.model,
            role: ROLE_TECHNICAL,
            label: "Technical Analyst",
            subject,
            question,
            evidence_block: &evidence_block,
            prior_debate: "(none)",
            max_tokens,
        }),
        run_agent_turn(AgentTurnArgs {
            session: &session,
            model: &req.model,
            role: ROLE_FUNDAMENTAL,
            label: "Fundamental Analyst",
            subject,
            question,
            evidence_block: &evidence_block,
            prior_debate: "(none)",
            max_tokens,
        }),
    );
    let technical = technical?;
    let fundamental = fundamental?;

    // Phase 2: bull vs bear debate, each seeing the analysts' findings.
    let analyst_summary = format!(
        "TECHNICAL: {}\n\nFUNDAMENTAL: {}",
        technical.analysis, fundamental.analysis
    );
    let (bull, bear) = tokio::join!(
        run_agent_turn(AgentTurnArgs {
            session: &session,
            model: &req.model,
            role: ROLE_BULL,
            label: "Bull Researcher",
            subject,
            question,
            evidence_block: &evidence_block,
            prior_debate: &analyst_summary,
            max_tokens,
        }),
        run_agent_turn(AgentTurnArgs {
            session: &session,
            model: &req.model,
            role: ROLE_BEAR,
            label: "Bear Researcher (Adversarial)",
            subject,
            question,
            evidence_block: &evidence_block,
            prior_debate: &analyst_summary,
            max_tokens,
        }),
    );
    let bull = bull?;
    let bear = bear?;

    // Phase 3: risk manager evaluates the debate.
    let debate_summary = format!(
        "{analyst_summary}\n\nBULL: {}\n\nBEAR: {}",
        bull.analysis, bear.analysis
    );
    let risk = run_agent_turn(AgentTurnArgs {
        session: &session,
        model: &req.model,
        role: ROLE_RISK,
        label: "Risk Manager",
        subject,
        question,
        evidence_block: &evidence_block,
        prior_debate: &debate_summary,
        max_tokens,
    })
    .await?;
    let risk_approved = risk.stance != Stance::Short || risk.conviction < 0.7;
    let risk_caveats: Vec<String> = if risk_approved {
        vec![]
    } else {
        vec!["Risk manager flagged elevated downside risk.".into()]
    };

    // Phase 4: PM synthesizes.
    let full_debate = format!("{debate_summary}\n\nRISK MANAGER: {}", risk.analysis);
    let pm = run_agent_turn(AgentTurnArgs {
        session: &session,
        model: &req.model,
        role: ROLE_PM,
        label: "Portfolio Manager",
        subject,
        question,
        evidence_block: &evidence_block,
        prior_debate: &full_debate,
        max_tokens,
    })
    .await?;

    let turns = vec![technical, fundamental, bull, bear, risk.clone(), pm.clone()];

    // Extract suggested levels from PM analysis if present.
    let (suggested_entry, suggested_stop, suggested_target) = extract_levels(&turns[5].analysis);

    let recommendation = AgentRecommendation {
        stance: pm.stance,
        conviction: pm.conviction,
        rationale: turns[5]
            .analysis
            .lines()
            .take_while(|l| !l.to_ascii_lowercase().contains("stance:"))
            .collect::<Vec<_>>()
            .join("\n"),
        suggested_entry,
        suggested_stop,
        suggested_target,
        risk_approved,
        risk_caveats,
        agents_consulted: turns.len(),
    };

    let generated_at = SystemClock::new().now().to_string();

    Ok(AgentCouncilResult {
        subject: req.subject,
        question: req.question,
        turns,
        recommendation,
        evidence_count: evidence_ids.len(),
        evidence_ids,
        generated_at,
        execution_eligible: false, // ALWAYS false — operator approves, risk gate enforces
        message: "Agent council debate complete. Recommendation is advisory only — never auto-executed. Operator approves; risk gate (1%/trade + 10% drawdown kill switch) enforces.".into(),
    })
}

fn extract_levels(text: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
    let lower = text.to_ascii_lowercase();
    let find_num = |key: &str| -> Option<f64> {
        lower.find(key).and_then(|i| {
            let rest = &text[i + key.len()..];
            rest.trim_start_matches(':')
                .trim_start()
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .next()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .filter(|v| *v > 0.0)
        })
    };
    (
        find_num("entry"),
        find_num("stop"),
        find_num("target"),
    )
}

fn durable_evidence_summary(evidence: &[crate::evidence_store::ModelEvidenceRecord]) -> String {
    evidence
        .iter()
        .take(10)
        .map(|e| {
            let payload_preview = e
                .normalized_payload
                .as_ref()
                .map(|v| {
                    let s = v.to_string();
                    if s.len() > 200 {
                        format!("{}…", &s[..200])
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| e.media_type.clone());
            format!("- {} ({}): {}", e.source_id, e.evidence_id, payload_preview)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_stance_parses_long() {
        let (s, c) = extract_stance("Bullish on BTC.\nSTANCE: long\nCONVICTION: 0.72");
        assert_eq!(s, Stance::Long);
        assert!((c - 0.72).abs() < 1e-9);
    }

    #[test]
    fn extract_stance_parses_short() {
        let (s, _) = extract_stance("Bearish.\nstance:short\nCONVICTION:0.6");
        assert_eq!(s, Stance::Short);
    }

    #[test]
    fn extract_stance_defaults_neutral() {
        let (s, c) = extract_stance("Unclear.\nSTANCE: neutral\nCONVICTION: 0.3");
        assert_eq!(s, Stance::Neutral);
        assert!((c - 0.3).abs() < 1e-9);
    }

    #[test]
    fn extract_stance_clamps_conviction() {
        let (_, c) = extract_stance("STANCE: long\nCONVICTION: 5.0");
        assert_eq!(c, 1.0);
    }

    #[test]
    fn extract_stance_missing_conviction_defaults_half() {
        let (_, c) = extract_stance("STANCE: long");
        assert_eq!(c, 0.5);
    }

    #[test]
    fn extract_levels_finds_entry_stop_target() {
        let text = "Entry: 45000\nStop: 44000\nTarget: 47000";
        let (e, s, t) = extract_levels(text);
        assert!((e.unwrap() - 45000.0).abs() < 1e-9);
        assert!((s.unwrap() - 44000.0).abs() < 1e-9);
        assert!((t.unwrap() - 47000.0).abs() < 1e-9);
    }

    #[test]
    fn extract_levels_returns_none_when_absent() {
        let (e, s, t) = extract_levels("No levels mentioned.");
        assert!(e.is_none());
        assert!(s.is_none());
        assert!(t.is_none());
    }
}
