//! Small app-local JSON stores for Wave 1 desktop preferences.

use std::{fs, path::PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerFilter {
    pub id: String,
    pub name: String,
    pub query: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub id: String,
    pub symbol: String,
    pub change_24h_above: f64,
    pub enabled: bool,
}

fn store_path(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("resolve app data directory: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| format!("create app data directory: {error}"))?;
    Ok(dir.join(file_name))
}

fn read_json<T>(app: &AppHandle, file_name: &str) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    let path = store_path(app, file_name)?;
    if !path.exists() {
        return Ok(T::default());
    }
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_json<T>(app: &AppHandle, file_name: &str, value: &T) -> Result<(), String>
where
    T: Serialize + ?Sized,
{
    let path = store_path(app, file_name)?;
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    fs::write(&path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

#[tauri::command]
pub fn get_watchlist(app: AppHandle) -> Result<Option<Vec<String>>, String> {
    let path = store_path(&app, "watchlist.json")?;
    if !path.exists() {
        return Ok(None);
    }
    read_json(&app, "watchlist.json").map(Some)
}

#[tauri::command]
pub fn save_watchlist(app: AppHandle, coingecko_ids: Vec<String>) -> Result<(), String> {
    write_json(&app, "watchlist.json", &coingecko_ids)
}

#[tauri::command]
pub fn get_scanner_filters(app: AppHandle) -> Result<Vec<ScannerFilter>, String> {
    read_json(&app, "scanner_filters.json")
}

#[tauri::command]
pub fn save_scanner_filters(app: AppHandle, filters: Vec<ScannerFilter>) -> Result<(), String> {
    write_json(&app, "scanner_filters.json", &filters)
}

#[tauri::command]
pub fn list_alert_rules(app: AppHandle) -> Result<Vec<AlertRule>, String> {
    read_json(&app, "alert_rules.json")
}

#[tauri::command]
pub fn upsert_alert_rule(app: AppHandle, rule: AlertRule) -> Result<Vec<AlertRule>, String> {
    let mut rules: Vec<AlertRule> = read_json(&app, "alert_rules.json")?;
    if let Some(existing) = rules.iter_mut().find(|candidate| candidate.id == rule.id) {
        *existing = rule;
    } else {
        rules.push(rule);
    }
    write_json(&app, "alert_rules.json", &rules)?;
    Ok(rules)
}

#[tauri::command]
pub fn get_data_mode() -> String {
    match std::env::var("PRISMATIK_DATA_MODE")
        .unwrap_or_else(|_| "cassette".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "live" => "live".into(),
        "chaos" | "degraded" => "chaos".into(),
        _ => "cassette".into(),
    }
}
