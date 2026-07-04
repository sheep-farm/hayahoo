use crate::utils::*;
use hayashi_plugin_sdk::arrow::array::ArrayRef;
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use serde_json::Value;
use std::collections::HashMap;

fn quote_raw(ticker: &str) -> Result<Value, String> {
    let path = "/v7/finance/quote";
    let params = [("symbols", ticker)];
    let json = yahoo_request(path, &params)?;
    let result = json["quoteResponse"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("ticker not found")?
        .clone();
    Ok(result)
}

/// Fetch a live quote for a single ticker. Returns a dictionary.
#[hayashi_fn]
pub fn quote(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let q = quote_raw(&ticker)?;
    Ok(json_object_to_dict(&q))
}

/// Fetch live quotes for multiple tickers. Returns a DataFrame.
#[hayashi_fn]
pub fn quotes(tickers: Vec<String>) -> Result<ArrayRef, String> {
    let symbols = tickers.join(",");
    let path = "/v7/finance/quote";
    let params = [("symbols", symbols.as_str())];
    let json = yahoo_request(path, &params)?;
    let results = json["quoteResponse"]["result"]
        .as_array()
        .ok_or("missing quote result")?;

    let fields = vec![
        "symbol",
        "shortName",
        "longName",
        "currency",
        "regularMarketPrice",
        "regularMarketChange",
        "regularMarketChangePercent",
        "regularMarketPreviousClose",
        "regularMarketOpen",
        "regularMarketDayHigh",
        "regularMarketDayLow",
        "regularMarketVolume",
        "fiftyTwoWeekHigh",
        "fiftyTwoWeekLow",
        "trailingPE",
        "forwardPE",
        "dividendYield",
    ];

    let mut columns: Vec<(&str, Vec<HayashiValue>)> =
        fields.iter().map(|&f| (f, Vec::with_capacity(results.len()))).collect();

    for result in results {
        for (i, field) in fields.iter().enumerate() {
            let value = json_value_to_hayashi(&result[*field]);
            columns[i].1.push(value);
        }
    }

    build_mixed_df(columns)
}

/// Quick helper: current price for a ticker.
#[hayashi_fn]
pub fn price(ticker: String) -> Result<f64, String> {
    let q = quote_raw(&ticker)?;
    q["regularMarketPrice"]
        .as_f64()
        .ok_or("price not available".to_string())
}

/// Quick helper: currency for a ticker.
#[hayashi_fn]
pub fn currency(ticker: String) -> Result<String, String> {
    let q = quote_raw(&ticker)?;
    Ok(json_str(&q, "currency"))
}

/// Quick helper: full name for a ticker.
#[hayashi_fn]
pub fn name(ticker: String) -> Result<String, String> {
    let q = quote_raw(&ticker)?;
    Ok(json_str(&q, "longName"))
}

/// Quick helper: 52-week high and low.
#[hayashi_fn]
pub fn fifty_two_week(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let q = quote_raw(&ticker)?;
    let mut dict = HashMap::new();
    if let Some(h) = q["fiftyTwoWeekHigh"].as_f64() {
        dict.insert("high".to_string(), HayashiValue::Float(h));
    }
    if let Some(l) = q["fiftyTwoWeekLow"].as_f64() {
        dict.insert("low".to_string(), HayashiValue::Float(l));
    }
    Ok(dict)
}
