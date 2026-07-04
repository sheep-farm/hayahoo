use crate::utils::*;
use hayashi_plugin_sdk::arrow::array::ArrayRef;
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use serde_json::Value;
use std::collections::HashMap;

/// Fetch the v8/finance/chart metadata for a ticker. This is the same endpoint
/// used by `waybar-tickers` and works without a Yahoo session crumb.
fn chart_meta(ticker: &str) -> Result<Value, String> {
    let path = format!("/v8/finance/chart/{}", ticker);
    let params = [("interval", "1d"), ("range", "2d")];
    let json = yahoo_request_public(&path, &params)?;
    let result = json["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("no chart data returned")?
        .clone();
    Ok(result)
}

fn meta_dict(meta: &Value) -> HashMap<String, HayashiValue> {
    json_object_to_dict(meta)
}

/// Fetch a live quote for a single ticker. Returns a dictionary built from the
/// chart metadata (price, currency, previous close, etc.).
#[hayashi_fn]
pub fn quote(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let chart = chart_meta(&ticker)?;
    Ok(meta_dict(&chart["meta"]))
}

/// Current market price for a ticker.
#[hayashi_fn]
pub fn price(ticker: String) -> Result<f64, String> {
    let chart = chart_meta(&ticker)?;
    chart["meta"]["regularMarketPrice"]
        .as_f64()
        .ok_or("price not available".to_string())
}

/// Currency for a ticker.
#[hayashi_fn]
pub fn currency(ticker: String) -> Result<String, String> {
    let chart = chart_meta(&ticker)?;
    Ok(json_str(&chart["meta"], "currency"))
}

/// Full name for a ticker.
#[hayashi_fn]
pub fn name(ticker: String) -> Result<String, String> {
    let chart = chart_meta(&ticker)?;
    let meta = &chart["meta"];
    let name = json_str(meta, "longName");
    if !name.is_empty() {
        Ok(name)
    } else {
        Ok(json_str(meta, "shortName"))
    }
}

/// Percentage change for a ticker (regularMarketPrice vs chartPreviousClose).
#[hayashi_fn]
pub fn change(ticker: String) -> Result<f64, String> {
    let chart = chart_meta(&ticker)?;
    let meta = &chart["meta"];
    let current = meta["regularMarketPrice"]
        .as_f64()
        .ok_or("current price not available")?;
    let previous = meta["chartPreviousClose"]
        .as_f64()
        .ok_or("previous close not available")?;
    Ok((current - previous) / previous * 100.0)
}

/// 52-week high and low for a ticker.
#[hayashi_fn]
pub fn fifty_two_week(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let chart = chart_meta(&ticker)?;
    let meta = &chart["meta"];
    let mut dict = HashMap::new();
    if let Some(h) = meta["fiftyTwoWeekHigh"].as_f64() {
        dict.insert("high".to_string(), HayashiValue::Float(h));
    }
    if let Some(l) = meta["fiftyTwoWeekLow"].as_f64() {
        dict.insert("low".to_string(), HayashiValue::Float(l));
    }
    Ok(dict)
}

/// Fetch live quotes for multiple tickers. Returns a DataFrame.
#[hayashi_fn]
pub fn quotes(tickers: Vec<String>) -> Result<ArrayRef, String> {
    let mut rows: Vec<HashMap<String, HayashiValue>> = Vec::with_capacity(tickers.len());
    for ticker in &tickers {
        let chart = chart_meta(ticker)?;
        let meta = &chart["meta"];
        let mut row = HashMap::new();
        row.insert("symbol".to_string(), HayashiValue::Str(ticker.clone()));
        row.insert(
            "price".to_string(),
            HayashiValue::Float(meta["regularMarketPrice"].as_f64().unwrap_or(0.0)),
        );
        row.insert(
            "currency".to_string(),
            HayashiValue::Str(json_str(meta, "currency")),
        );
        row.insert(
            "name".to_string(),
            HayashiValue::Str({
                let n = json_str(meta, "longName");
                if !n.is_empty() { n } else { json_str(meta, "shortName") }
            }),
        );
        let change_pct = match (
            meta["regularMarketPrice"].as_f64(),
            meta["chartPreviousClose"].as_f64(),
        ) {
            (Some(curr), Some(prev)) => HayashiValue::Float((curr - prev) / prev * 100.0),
            _ => HayashiValue::Float(0.0),
        };
        row.insert("change_pct".to_string(), change_pct);
        row.insert(
            "previous_close".to_string(),
            HayashiValue::Float(meta["chartPreviousClose"].as_f64().unwrap_or(0.0)),
        );
        row.insert(
            "volume".to_string(),
            HayashiValue::Float(meta["regularMarketVolume"].as_f64().unwrap_or(0.0)),
        );
        rows.push(row);
    }

    let mut columns: Vec<(&str, Vec<HayashiValue>)> = vec![
        ("symbol", Vec::with_capacity(rows.len())),
        ("price", Vec::with_capacity(rows.len())),
        ("currency", Vec::with_capacity(rows.len())),
        ("name", Vec::with_capacity(rows.len())),
        ("change_pct", Vec::with_capacity(rows.len())),
        ("previous_close", Vec::with_capacity(rows.len())),
        ("volume", Vec::with_capacity(rows.len())),
    ];

    for mut row in rows {
        columns[0].1.push(row.remove("symbol").unwrap_or(HayashiValue::Str(String::new())));
        columns[1].1.push(row.remove("price").unwrap_or(HayashiValue::Float(0.0)));
        columns[2].1.push(row.remove("currency").unwrap_or(HayashiValue::Str(String::new())));
        columns[3].1.push(row.remove("name").unwrap_or(HayashiValue::Str(String::new())));
        columns[4].1.push(row.remove("change_pct").unwrap_or(HayashiValue::Float(0.0)));
        columns[5].1.push(row.remove("previous_close").unwrap_or(HayashiValue::Float(0.0)));
        columns[6].1.push(row.remove("volume").unwrap_or(HayashiValue::Float(0.0)));
    }

    build_mixed_df(columns)
}
