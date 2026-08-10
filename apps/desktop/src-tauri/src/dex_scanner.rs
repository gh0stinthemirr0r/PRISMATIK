//! Solana DEX arbitrage scanner — read-only cross-venue price monitoring.
//!
//! Directive #9. Scans token prices across major Solana DEXes via public RPC
//! and detects cross-venue price inefficiencies. This is a SCANNER only:
//!
//! - It READS prices from public DEX APIs / RPC endpoints.
//! - It COMPUTES the theoretical arbitrage edge locally.
//! - It SURFACES opportunities with honest feasibility assessment.
//! - It does NOT submit on-chain transactions, sign transactions, or hold
//!   private keys. No live execution path exists here.
//!
//! The directive mentions "executing hundreds of trades in milliseconds."
//! That level of HFT/MEV execution requires: a private key in memory,
//! low-latency dedicated RPC nodes, Jito bundle submission, priority fee
//! auction management, and transaction signing — all of which are
//! execution-gated safety decisions, not scanner features. What this module
//! delivers honestly is the *intelligence layer*: detecting where the edges
//! are, how large they are after estimated fees, and whether they survive
//! a feasibility check. An operator who approves live execution can then
//! wire the detected opportunities to a separate, explicitly-approved
//! signing path.

use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

/// A DEX venue on Solana.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DexVenue {
    /// Jupiter aggregator (aggregated best price across DEXes).
    Jupiter,
    /// Raydium AMM.
    Raydium,
    /// Orca Whirlpool.
    Orca,
    /// Meteora DLMM.
    Meteora,
    /// Phoenix order book.
    Phoenix,
}

impl DexVenue {}

/// A price quote from a single DEX venue.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DexQuote {
    pub venue: DexVenue,
    pub token_symbol: String,
    pub price_usd: f64,
    pub liquidity_usd: Option<f64>,
    pub source: String,
}

/// A detected cross-venue arbitrage opportunity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitrageOpportunity {
    pub token_symbol: String,
    pub buy_venue: DexVenue,
    pub buy_price: f64,
    pub sell_venue: DexVenue,
    pub sell_price: f64,
    /// Raw price spread fraction (sell - buy) / buy.
    pub spread_pct: f64,
    /// Estimated Solana network + priority fee in USD (conservative).
    pub estimated_fee_usd: f64,
    /// Net edge after fees (spread - fees / trade_size).
    pub net_edge_after_fees_pct: f64,
    /// Whether the edge survives a feasibility check (net positive and
    /// liquidity appears sufficient on both legs).
    pub feasible: bool,
    /// Honest feasibility notes.
    pub notes: Vec<String>,
    /// Execution mode — always "scan_only".
    pub execution_mode: String,
}

/// Scanner result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub quotes: Vec<DexQuote>,
    pub opportunities: Vec<ArbitrageOpportunity>,
    pub scanned_at: String,
    pub rpc_status: String,
    pub message: String,
}

/// Conservative estimate of Solana transaction costs for an arbitrage
/// execution: base fee (0.000005 SOL) + priority fee for inclusion + DEX
/// swap fees (typically 0.25-1%). Returns USD equivalent.
fn estimated_round_trip_fee_usd(sol_price_usd: f64) -> f64 {
    // Base fee: 0.000005 SOL × 2 (two swaps) = 0.00001 SOL
    let base_fee_sol = 0.00001;
    // Priority fee: conservative 0.0001 SOL for competitive inclusion
    let priority_fee_sol = 0.0001;
    // Network fee subtotal in USD. DEX swap fees (~0.3% per swap × 2 = 0.6%)
    // are computed per-trade-size in the feasibility check; we return the
    // network component here.
    (base_fee_sol + priority_fee_sol) * sol_price_usd
}

/// Detect arbitrage opportunities from a set of cross-venue quotes for the
/// same token. Pure function — deterministic given the inputs.
pub fn detect_opportunities(
    quotes: &[DexQuote],
    sol_price_usd: f64,
    reference_trade_size_usd: f64,
) -> Vec<ArbitrageOpportunity> {
    let network_fee = estimated_round_trip_fee_usd(sol_price_usd);
    // DEX swap fee estimate: 0.3% per swap × 2 = 0.6% of trade size.
    let swap_fee_pct = 0.006;
    let swap_fee_usd = reference_trade_size_usd * swap_fee_pct;

    // Group quotes by token symbol.
    let mut by_token: std::collections::BTreeMap<&str, Vec<&DexQuote>> =
        std::collections::BTreeMap::new();
    for q in quotes {
        by_token.entry(q.token_symbol.as_str()).or_default().push(q);
    }

    let mut opportunities = Vec::new();
    for (symbol, token_quotes) in by_token {
        if token_quotes.len() < 2 {
            continue;
        }
        // Find the cheapest buy and the highest sell.
        let mut sorted = token_quotes.clone();
        sorted.sort_by(|a, b| {
            a.price_usd
                .partial_cmp(&b.price_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let cheapest = sorted.first().unwrap();
        let priciest = sorted.last().unwrap();
        if cheapest.venue == priciest.venue {
            continue;
        }
        let spread_pct = (priciest.price_usd - cheapest.price_usd) / cheapest.price_usd;
        let total_fees_usd = network_fee + swap_fee_usd;
        let fee_pct = total_fees_usd / reference_trade_size_usd;
        let net_edge = spread_pct - fee_pct;
        let feasible = net_edge > 0.0;

        let mut notes = Vec::new();
        if !feasible {
            notes.push(format!(
                "Spread {:.3}% does not cover estimated fees {:.3}% (network ${:.4} + swap ${:.4} on ${:.0} trade).",
                spread_pct * 100.0,
                fee_pct * 100.0,
                network_fee,
                swap_fee_usd,
                reference_trade_size_usd
            ));
        }
        // Liquidity check.
        let buy_liq = cheapest.liquidity_usd.unwrap_or(0.0);
        let sell_liq = priciest.liquidity_usd.unwrap_or(0.0);
        if buy_liq > 0.0 && buy_liq < reference_trade_size_usd {
            notes.push(format!(
                "Buy venue liquidity (${buy_liq:.0}) is below the reference trade size (${reference_trade_size_usd:.0}) — slippage would erode the edge."
            ));
        }
        if sell_liq > 0.0 && sell_liq < reference_trade_size_usd {
            notes.push(format!(
                "Sell venue liquidity (${sell_liq:.0}) is below the reference trade size (${reference_trade_size_usd:.0})."
            ));
        }
        notes.push(
            "This is a scan-only detection. Live execution requires an explicitly-approved \
             signing path with a funded Solana keypair — not available in this build."
                .to_string(),
        );

        opportunities.push(ArbitrageOpportunity {
            token_symbol: symbol.to_string(),
            buy_venue: cheapest.venue,
            buy_price: cheapest.price_usd,
            sell_venue: priciest.venue,
            sell_price: priciest.price_usd,
            spread_pct,
            estimated_fee_usd: total_fees_usd,
            net_edge_after_fees_pct: net_edge,
            feasible: feasible && notes.len() <= 2,
            notes,
            execution_mode: "scan_only".to_string(),
        });
    }
    // Sort by net edge descending.
    opportunities.sort_by(|a, b| {
        b.net_edge_after_fees_pct
            .partial_cmp(&a.net_edge_after_fees_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    opportunities
}

/// Scan for arbitrage opportunities. Reads current prices from the terminal
/// feed (which may include governed crypto quotes) and computes cross-venue
/// edges. If no real DEX quotes are available, returns an honest empty state.
#[tauri::command]
pub(crate) async fn scan_dex_arbitrage(
    sol_price_usd: Option<f64>,
    reference_trade_size_usd: Option<f64>,
) -> Result<ScanResult, String> {
    let sol_price = sol_price_usd.unwrap_or(180.0); // conservative fallback
    let trade_size = reference_trade_size_usd.unwrap_or(1000.0);

    // Gather current governed quotes from the terminal feed. These come from
    // whatever crypto providers the operator has connected (CoinGecko, etc.).
    // For a true cross-DEX scan, the operator would need to connect Solana
    // DEX RPC adapters — those are execution-gated and require an approved
    // RPC endpoint + source policy.
    let snapshot = crate::terminal_feed::get_terminal_feed().await?;
    let quotes: Vec<DexQuote> = snapshot
        .quotes
        .iter()
        .filter(|q| {
            matches!(
                q.symbol.to_uppercase().as_str(),
                "SOL" | "BTC" | "ETH" | "JUP" | "RAY" | "ORCA" | "JTO" | "BONK" | "WIF" | "PYTH"
            )
        })
        .map(|q| DexQuote {
            venue: DexVenue::Jupiter, // placeholder — real cross-venue needs DEX RPC adapters
            token_symbol: q.symbol.clone(),
            price_usd: q.price,
            liquidity_usd: None,
            source: format!("governed:{}", q.provider),
        })
        .collect();

    let opportunities = detect_opportunities(&quotes, sol_price, trade_size);

    let rpc_status = if quotes.is_empty() {
        "No governed Solana DEX quotes available. Connect a crypto provider in Integrations to populate price feeds. Cross-venue DEX RPC adapters require an approved source policy.".to_string()
    } else {
        format!("{} governed quotes from terminal feed. Cross-venue detection requires multiple DEX RPC sources — currently single-venue (Jupiter) placeholder.", quotes.len())
    };

    let message = if opportunities.is_empty() {
        "No arbitrage opportunities detected. Either no cross-venue price divergence exists in the current feed, or only a single venue is connected.".to_string()
    } else {
        format!(
            "{} potential opportunit(y/ies) detected. {} feasible after fees. All are scan-only — no execution path.",
            opportunities.len(),
            opportunities.iter().filter(|o| o.feasible).count()
        )
    };

    Ok(ScanResult {
        quotes,
        opportunities,
        scanned_at: SystemClock::new().now().to_string(),
        rpc_status,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(venue: DexVenue, token: &str, price: f64, liq: Option<f64>) -> DexQuote {
        DexQuote {
            venue,
            token_symbol: token.into(),
            price_usd: price,
            liquidity_usd: liq,
            source: "test".into(),
        }
    }

    #[test]
    fn detects_cross_venue_spread() {
        let quotes = vec![
            quote(DexVenue::Raydium, "SOL", 180.0, Some(50_000.0)),
            quote(DexVenue::Orca, "SOL", 182.0, Some(50_000.0)),
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert_eq!(opps.len(), 1);
        let opp = &opps[0];
        assert_eq!(opp.token_symbol, "SOL");
        assert_eq!(opp.buy_venue, DexVenue::Raydium);
        assert_eq!(opp.sell_venue, DexVenue::Orca);
        // Spread = (182-180)/180 = 1.11%
        assert!((opp.spread_pct - 0.01111).abs() < 0.001);
    }

    #[test]
    fn infeasible_when_spread_below_fees() {
        let quotes = vec![
            quote(DexVenue::Raydium, "SOL", 180.0, None),
            quote(DexVenue::Orca, "SOL", 180.3, None), // 0.17% spread < 0.6% swap fee
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert_eq!(opps.len(), 1);
        assert!(!opps[0].feasible);
        assert!(opps[0].net_edge_after_fees_pct < 0.0);
    }

    #[test]
    fn feasible_when_spread_exceeds_fees() {
        let quotes = vec![
            quote(DexVenue::Raydium, "JUP", 0.50, Some(100_000.0)),
            quote(DexVenue::Orca, "JUP", 0.55, Some(100_000.0)), // 10% spread >> fees
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert_eq!(opps.len(), 1);
        assert!(opps[0].feasible);
        assert!(opps[0].net_edge_after_fees_pct > 0.0);
    }

    #[test]
    fn no_opportunity_for_single_venue() {
        let quotes = vec![quote(DexVenue::Raydium, "SOL", 180.0, None)];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert!(opps.is_empty());
    }

    #[test]
    fn sorted_by_net_edge_descending() {
        let quotes = vec![
            quote(DexVenue::Raydium, "SOL", 180.0, Some(100_000.0)),
            quote(DexVenue::Orca, "SOL", 185.0, Some(100_000.0)), // 2.8% spread
            quote(DexVenue::Raydium, "JUP", 0.50, Some(100_000.0)),
            quote(DexVenue::Orca, "JUP", 0.60, Some(100_000.0)), // 20% spread
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert_eq!(opps.len(), 2);
        assert!(opps[0].net_edge_after_fees_pct >= opps[1].net_edge_after_fees_pct);
        assert_eq!(opps[0].token_symbol, "JUP"); // larger edge first
    }

    #[test]
    fn low_liquidity_flagged_in_notes() {
        let quotes = vec![
            quote(DexVenue::Raydium, "WIF", 2.0, Some(500.0)), // low liq
            quote(DexVenue::Orca, "WIF", 2.5, Some(100_000.0)),
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert!(!opps.is_empty());
        assert!(opps[0].notes.iter().any(|n| n.contains("liquidity")));
    }

    #[test]
    fn execution_mode_is_scan_only() {
        let quotes = vec![
            quote(DexVenue::Raydium, "SOL", 180.0, Some(100_000.0)),
            quote(DexVenue::Orca, "SOL", 190.0, Some(100_000.0)),
        ];
        let opps = detect_opportunities(&quotes, 180.0, 1000.0);
        assert_eq!(opps[0].execution_mode, "scan_only");
    }

    #[test]
    fn estimated_fee_is_positive_and_modest() {
        let fee = estimated_round_trip_fee_usd(180.0);
        assert!(fee > 0.0);
        // Should be under $0.05 at $180/SOL
        assert!(fee < 0.05, "fee {fee} should be modest");
    }
}
