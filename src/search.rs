use crate::utils::*;
use hayashi_plugin_sdk::arrow::array::ArrayRef;
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use std::collections::HashMap;

/// Search Yahoo Finance for tickers, companies, ETFs, indices, etc.
/// Options:
///   - `limit`: maximum number of results (default 10, max 50)
///   - `quotes`: include quotes in results (default true)
///   - `news`: include news in results (default false)
#[hayashi_fn]
pub fn search(
    query: String,
    options: HashMap<String, HayashiValue>,
) -> Result<ArrayRef, String> {
    let limit = opt_i64(&options, "limit").unwrap_or(10).clamp(1, 50).to_string();
    let quotes = opt_str(&options, "quotes").unwrap_or_else(|| "true".to_string());
    let news = opt_str(&options, "news").unwrap_or_else(|| "false".to_string());

    let path = "/v1/finance/search";
    let params = [
        ("q", query.as_str()),
        ("quotesCount", limit.as_str()),
        ("quotesQueryId", if quotes == "true" { "tss_match_phrase" } else { "" }),
        ("newsCount", if news == "true" { "10" } else { "0" }),
    ];

    let json = yahoo_request_public(path, &params)?;
    let results = json["quotes"].as_array().ok_or("missing quotes")?;

    let columns: Vec<(&str, Vec<String>)> = vec![
        ("symbol", results.iter().map(|r| json_str(r, "symbol")).collect::<Vec<_>>()),
        ("shortname", results.iter().map(|r| json_str(r, "shortname")).collect::<Vec<_>>()),
        ("longname", results.iter().map(|r| json_str(r, "longname")).collect::<Vec<_>>()),
        ("exch", results.iter().map(|r| json_str(r, "exch")).collect::<Vec<_>>()),
        ("type", results.iter().map(|r| json_str(r, "type")).collect::<Vec<_>>()),
        ("typeDisp", results.iter().map(|r| json_str(r, "typeDisp")).collect::<Vec<_>>()),
        ("sector", results.iter().map(|r| json_str(r, "sector")).collect::<Vec<_>>()),
        ("industry", results.iter().map(|r| json_str(r, "industry")).collect::<Vec<_>>()),
    ];

    let mut mixed: Vec<(&str, Vec<HayashiValue>)> = columns
        .into_iter()
        .map(|(name, vals)| (name, vals.into_iter().map(HayashiValue::Str).collect::<Vec<_>>()))
        .collect();

    if let Some(exchange_arr) = json["exchanges"].as_array() {
        let mut exchange_map: HashMap<String, String> = HashMap::new();
        for ex in exchange_arr {
            if let (Some(code), Some(name)) = (ex["exchCode"].as_str(), ex["name"].as_str()) {
                exchange_map.insert(code.to_string(), name.to_string());
            }
        }
        let names: Vec<HayashiValue> = results
            .iter()
            .map(|r| {
                let code = json_str(r, "exch");
                exchange_map.get(&code).cloned().unwrap_or(code)
            })
            .map(HayashiValue::Str)
            .collect();
        mixed.push(("exchange", names));
    }

    build_mixed_df(mixed)
}

/// Search for a single ticker and return the best match symbol.
#[hayashi_fn]
pub fn search_symbol(query: String) -> Result<String, String> {
    let mut opts = HashMap::new();
    opts.insert("limit".to_string(), HayashiValue::Int(1));
    let path = "/v1/finance/search";
    let params = [
        ("q", query.as_str()),
        ("quotesCount", "1"),
        ("quotesQueryId", "tss_match_phrase"),
    ];
    let json = yahoo_request_public(path, &params)?;
    let result = json["quotes"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("no results")?;
    Ok(json_str(result, "symbol"))
}
