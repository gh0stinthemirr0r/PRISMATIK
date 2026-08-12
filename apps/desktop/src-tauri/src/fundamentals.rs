use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FinancialStatement {
    pub(crate) filing_date: String,
    pub(crate) form: String,
    pub(crate) period: String,
    pub(crate) revenue: Option<f64>,
    pub(crate) net_income: Option<f64>,
    pub(crate) eps: Option<f64>,
    pub(crate) total_assets: Option<f64>,
    pub(crate) total_liabilities: Option<f64>,
    pub(crate) shareholders_equity: Option<f64>,
    pub(crate) operating_cash_flow: Option<f64>,
    pub(crate) free_cash_flow: Option<f64>,
    pub(crate) source_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FundamentalsResult {
    pub(crate) cik: String,
    pub(crate) entity_name: String,
    pub(crate) statements: Vec<FinancialStatement>,
    pub(crate) source: String,
}

static COMPANY_CACHE: LazyLock<RwLock<BTreeMap<String, serde_json::Value>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Fetch company fundamentals from SEC EDGAR XBRL API (free, requires User-Agent).
#[tauri::command]
pub(crate) async fn get_fundamentals(
    cik: String,
    limit: Option<usize>,
) -> Result<FundamentalsResult, String> {
    let limit = limit.unwrap_or(20);
    let cik_padded = format!(
        "{:0>10}",
        cik.trim_start_matches("CIK").trim_start_matches('0')
    );

    let client = reqwest::Client::new();

    // get company facts (XBRL data)
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{cik_padded}.json");
    let resp = client
        .get(&url)
        .header("User-Agent", "PRISMATIK/0.5 research@mythos.systems")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("SEC EDGAR returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    let entity_name = val["entityName"].as_str().unwrap_or("Unknown").to_string();

    // extract key financial metrics from XBRL facts
    let facts = &val["facts"]["us-gaap"];

    let get_metric = |metric: &str| -> Vec<(String, f64)> {
        facts[metric]["units"]["USD"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let val = item["val"].as_f64()?;
                        let end = item["end"].as_str()?;
                        Some((end.to_string(), val))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut rev = get_metric("Revenues");
    if rev.is_empty() {
        rev = get_metric("RevenueFromContractWithCustomerExcludingAssessedTax");
    }
    let net_incomes = get_metric("NetIncomeLoss");
    let total_assets = get_metric("Assets");
    let total_liabilities = get_metric("Liabilities");
    let equity = get_metric("StockholdersEquity");
    let operating_cf = get_metric("NetCashProvidedByUsedInOperatingActivities");
    let capex = get_metric("PaymentsToAcquirePropertyPlantAndEquipment");

    // build statements from available data points
    let mut statements = Vec::new();
    let mut dates: Vec<String> = rev.iter().map(|(d, _)| d.clone()).collect();
    dates.extend(net_incomes.iter().map(|(d, _)| d.clone()));
    dates.sort();
    dates.dedup();
    dates.reverse();

    for date in dates.iter().take(limit) {
        let rev_val = rev.iter().find(|(d, _)| d == date).map(|(_, v)| *v);
        let ni = net_incomes.iter().find(|(d, _)| d == date).map(|(_, v)| *v);
        let ta = total_assets
            .iter()
            .find(|(d, _)| d == date)
            .map(|(_, v)| *v);
        let tl = total_liabilities
            .iter()
            .find(|(d, _)| d == date)
            .map(|(_, v)| *v);
        let eq = equity.iter().find(|(d, _)| d == date).map(|(_, v)| *v);
        let ocf = operating_cf
            .iter()
            .find(|(d, _)| d == date)
            .map(|(_, v)| *v);
        let cx = capex.iter().find(|(d, _)| d == date).map(|(_, v)| *v);

        let fcf = match (ocf, cx) {
            (Some(o), Some(c)) => Some(o - c.abs()),
            _ => None,
        };

        statements.push(FinancialStatement {
            filing_date: date.clone(),
            form: "10-K/10-Q".into(),
            period: if date.len() >= 10 {
                date[..7].to_string()
            } else {
                date.clone()
            },
            revenue: rev_val,
            net_income: ni,
            eps: None,
            total_assets: ta,
            total_liabilities: tl,
            shareholders_equity: eq,
            operating_cash_flow: ocf,
            free_cash_flow: fcf,
            source_url: format!(
                "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={cik_padded}"
            ),
        });
    }

    Ok(FundamentalsResult {
        cik: cik_padded,
        entity_name,
        statements,
        source: "SEC EDGAR XBRL".into(),
    })
}
