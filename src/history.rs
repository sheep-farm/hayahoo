use crate::utils::*;
use hayashi_plugin_sdk::arrow::array::{ArrayRef, Float64Array, StringArray, StructArray};
use hayashi_plugin_sdk::arrow::datatypes::{DataType, Field};
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Fetch historical OHLCV data for a ticker.
/// Options:
///   - `start` / `end`: YYYY-MM-DD window (optional, overrides `range`)
///   - `interval`: e.g. "1d", "1wk", "1mo" (default "1d")
///   - `range`: e.g. "1y", "5y", "max" (default "1y")
#[hayashi_fn]
pub fn history(
    ticker: String,
    options: HashMap<String, HayashiValue>,
) -> Result<ArrayRef, String> {
    let interval = opt_str(&options, "interval").unwrap_or_else(|| "1d".to_string());
    let range = opt_str(&options, "range").unwrap_or_else(|| "1y".to_string());
    let start = opt_str(&options, "start");
    let end = opt_str(&options, "end");

    let path = format!("/v8/finance/chart/{}", ticker);
    let mut params = vec![
        ("interval", interval.as_str()),
        ("range", range.as_str()),
    ];

    let period1: Option<i64> = start
        .as_ref()
        .and_then(|s| parse_date(s))
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp());
    let period2: Option<i64> = end
        .as_ref()
        .and_then(|e| parse_date(e))
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp());

    let period1_str: Option<String> = period1.map(|p| p.to_string());
    let period2_str: Option<String> = period2.map(|p| p.to_string());
    if let Some(ref p1) = period1_str {
        params.push(("period1", p1.as_str()));
    }
    if let Some(ref p2) = period2_str {
        params.push(("period2", p2.as_str()));
    }

    let json = yahoo_request(&path, &params)?;
    let result = json["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("no chart data returned")?;

    let timestamps = result["timestamp"].as_array().ok_or("missing timestamps")?;
    let quote = result["indicators"]["quote"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("missing quote indicators")?;

    let adjclose = result["indicators"]["adjclose"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|obj| obj["adjclose"].as_array());

    let mut dates: Vec<String> = Vec::with_capacity(timestamps.len());
    let mut open: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());
    let mut high: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());
    let mut low: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());
    let mut close: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());
    let mut adj_close: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());
    let mut volume: Vec<Option<f64>> = Vec::with_capacity(timestamps.len());

    for (i, ts) in timestamps.iter().enumerate() {
        let ts_ms = ts.as_i64().unwrap_or(0) * 1000;
        dates.push(timestamp_ms_to_date(ts_ms));

        open.push(array_f64(quote, "open", i));
        high.push(array_f64(quote, "high", i));
        low.push(array_f64(quote, "low", i));
        close.push(array_f64(quote, "close", i));
        volume.push(array_f64(quote, "volume", i));

        if let Some(adj) = adjclose.as_ref() {
            adj_close.push(adj.get(i).and_then(|v| v.as_f64()));
        } else {
            adj_close.push(close.last().copied().flatten());
        }
    }

    let fields: Vec<(Arc<Field>, ArrayRef)> = vec![
        (Arc::new(Field::new("date", DataType::Utf8, false)), Arc::new(StringArray::from(dates)) as ArrayRef),
        (Arc::new(Field::new("open", DataType::Float64, true)), Arc::new(Float64Array::from(open)) as ArrayRef),
        (Arc::new(Field::new("high", DataType::Float64, true)), Arc::new(Float64Array::from(high)) as ArrayRef),
        (Arc::new(Field::new("low", DataType::Float64, true)), Arc::new(Float64Array::from(low)) as ArrayRef),
        (Arc::new(Field::new("close", DataType::Float64, true)), Arc::new(Float64Array::from(close)) as ArrayRef),
        (Arc::new(Field::new("adj_close", DataType::Float64, true)), Arc::new(Float64Array::from(adj_close)) as ArrayRef),
        (Arc::new(Field::new("volume", DataType::Float64, true)), Arc::new(Float64Array::from(volume)) as ArrayRef),
    ];

    Ok(Arc::new(StructArray::from(fields)))
}

fn array_f64(value: &Value, key: &str, idx: usize) -> Option<f64> {
    value[key]
        .as_array()
        .and_then(|arr| arr.get(idx))
        .and_then(|v| v.as_f64())
}
