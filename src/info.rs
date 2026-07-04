use crate::utils::*;
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use std::collections::HashMap;

fn info_impl(
    ticker: String,
    options: HashMap<String, HayashiValue>,
) -> Result<HashMap<String, HayashiValue>, String> {
    let modules = opt_str(&options, "modules").unwrap_or_else(|| {
        "summaryProfile,financialData,defaultKeyStatistics,price".to_string()
    });

    let path = format!("/v11/finance/quoteSummary/{}", ticker);
    let params = [("modules", modules.as_str())];
    let json = yahoo_request(&path, &params)?;

    let result = json["quoteSummary"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or("no summary data returned")?;

    let mut dict = HashMap::new();
    for (module, value) in result.as_object().unwrap_or(&serde_json::Map::new()) {
        if let Some(data) = value
            .as_object()
            .and_then(|obj| obj.get("result"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
        {
            dict.insert(module.clone(), json_value_to_hayashi(data));
        }
    }

    Ok(dict)
}

/// Fetch summary information for a ticker.
/// Options:
///   - `modules`: comma-separated list of Yahoo quoteSummary modules
///     (default "summaryProfile,financialData,defaultKeyStatistics,price")
#[hayashi_fn]
pub fn info(
    ticker: String,
    options: HashMap<String, HayashiValue>,
) -> Result<HashMap<String, HayashiValue>, String> {
    info_impl(ticker, options)
}

/// Fetch key statistics for a ticker.
#[hayashi_fn]
pub fn key_statistics(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let mut opts = HashMap::new();
    opts.insert(
        "modules".to_string(),
        HayashiValue::Str("defaultKeyStatistics".to_string()),
    );
    info_impl(ticker, opts)
}

/// Fetch financial data for a ticker.
#[hayashi_fn]
pub fn financial_data(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let mut opts = HashMap::new();
    opts.insert(
        "modules".to_string(),
        HayashiValue::Str("financialData".to_string()),
    );
    info_impl(ticker, opts)
}

/// Fetch profile data for a ticker.
#[hayashi_fn]
pub fn profile(ticker: String) -> Result<HashMap<String, HayashiValue>, String> {
    let mut opts = HashMap::new();
    opts.insert(
        "modules".to_string(),
        HayashiValue::Str("summaryProfile".to_string()),
    );
    info_impl(ticker, opts)
}
