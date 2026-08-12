use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YieldCurvePoint {
    pub(crate) maturity: String,
    pub(crate) maturity_years: f64,
    pub(crate) yield_pct: f64,
    pub(crate) date: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YieldCurveResult {
    pub(crate) date: String,
    pub(crate) points: Vec<YieldCurvePoint>,
    pub(crate) spread_10y_2y: f64,
    pub(crate) spread_10y_3m: f64,
    pub(crate) curve_shape: String,
    pub(crate) source: String,
}

/// Fetch current US Treasury yield curve from FRED (free, no API key for this endpoint).
#[tauri::command]
pub(crate) async fn get_yield_curve() -> Result<YieldCurveResult, String> {
    let client = reqwest::Client::new();

    // Treasury yield series from FRED
    let series = [
        ("DGS1MO", "1M", 0.083),
        ("DGS3MO", "3M", 0.25),
        ("DGS6MO", "6M", 0.5),
        ("DGS1", "1Y", 1.0),
        ("DGS2", "2Y", 2.0),
        ("DGS3", "3Y", 3.0),
        ("DGS5", "5Y", 5.0),
        ("DGS7", "7Y", 7.0),
        ("DGS10", "10Y", 10.0),
        ("DGS20", "20Y", 20.0),
        ("DGS30", "30Y", 30.0),
    ];

    let mut points = Vec::new();

    for (series_id, label, years) in series {
        let url = format!(
            "https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}&cosd=2026-01-01"
        );
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    // parse CSV, take last value
                    let last = text.lines().last().unwrap_or("");
                    let value = last
                        .split(',')
                        .nth(1)
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    if value > 0.0 {
                        points.push(YieldCurvePoint {
                            maturity: label.to_string(),
                            maturity_years: years,
                            yield_pct: value,
                            date: last.split(',').next().unwrap_or("").to_string(),
                        });
                    }
                }
            },
            _ => {},
        }
    }

    if points.is_empty() {
        return Err("Could not fetch yield curve data from FRED".into());
    }

    // compute spreads
    let y10 = points
        .iter()
        .find(|p| p.maturity == "10Y")
        .map(|p| p.yield_pct)
        .unwrap_or(0.0);
    let y2 = points
        .iter()
        .find(|p| p.maturity == "2Y")
        .map(|p| p.yield_pct)
        .unwrap_or(0.0);
    let y3m = points
        .iter()
        .find(|p| p.maturity == "3M")
        .map(|p| p.yield_pct)
        .unwrap_or(0.0);

    let spread_10y_2y = y10 - y2;
    let spread_10y_3m = y10 - y3m;

    let curve_shape = if spread_10y_2y < -0.5 {
        "Deeply Inverted".to_string()
    } else if spread_10y_2y < 0.0 {
        "Inverted".to_string()
    } else if spread_10y_2y < 0.5 {
        "Flat".to_string()
    } else if spread_10y_2y < 1.5 {
        "Normal".to_string()
    } else {
        "Steep".to_string()
    };

    let date = points.first().map(|p| p.date.clone()).unwrap_or_default();

    Ok(YieldCurveResult {
        date,
        points,
        spread_10y_2y,
        spread_10y_3m,
        curve_shape,
        source: "FRED".into(),
    })
}

/// Fetch credit spread data (ICE BofA indices from FRED).
#[tauri::command]
pub(crate) async fn get_credit_spreads() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

    // ICE BofA US Corporate Index Option-Adjusted Spread
    let series = [
        ("BAMLC0A0CM", "IG Corporate"),
        ("BAMLH0A0HYM2", "HY Corporate"),
        ("BAMLC0A4CBBB", "BBB Corporate"),
        ("BAMLC0A1CAAA", "AAA Corporate"),
    ];

    let mut spreads = serde_json::Map::new();
    for (id, label) in series {
        let url =
            format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={id}&cosd=2024-01-01");
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    let last = text.lines().last().unwrap_or("");
                    let value = last
                        .split(',')
                        .nth(1)
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    spreads.insert(
                        label.to_string(),
                        serde_json::json!({
                            "series": id,
                            "spread": value,
                            "date": last.split(',').next().unwrap_or("")
                        }),
                    );
                }
            }
        }
    }

    Ok(serde_json::Value::Object(spreads))
}
