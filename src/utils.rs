use chrono::NaiveDate;
use hayashi_plugin_sdk::arrow::array::{ArrayRef, Float64Array, StringArray, StructArray};
use hayashi_plugin_sdk::arrow::datatypes::{DataType, Field};
use hayashi_plugin_sdk::hayashi_fn;
use hayashi_plugin_sdk::value::HayashiValue;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use ureq::Agent;

pub const YAHOO_BASE: &str = "https://query1.finance.yahoo.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

struct YahooState {
    agent: Agent,
    crumb: String,
}

static STATE: OnceLock<Mutex<YahooState>> = OnceLock::new();

fn state() -> &'static Mutex<YahooState> {
    STATE.get_or_init(|| {
        Mutex::new(YahooState {
            agent: ureq::agent(),
            crumb: String::new(),
        })
    })
}

fn ensure_session() -> Result<(Agent, String), String> {
    let guard = state().lock().unwrap();
    if !guard.crumb.is_empty() {
        return Ok((guard.agent.clone(), guard.crumb.clone()));
    }
    drop(guard);

    let mut guard = state().lock().unwrap();
    // Double-check after re-acquiring the lock.
    if !guard.crumb.is_empty() {
        return Ok((guard.agent.clone(), guard.crumb.clone()));
    }

    // Prime a session cookie.
    guard
        .agent
        .get("https://fc.yahoo.com/")
        .set("User-Agent", USER_AGENT)
        .set("Accept", "*/*")
        .call()
        .map_err(|e| format!("Yahoo session prime failed: {}", e))?;

    // Fetch crumb using the cookie jar now primed by fc.yahoo.com.
    let crumb = guard
        .agent
        .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
        .set("User-Agent", USER_AGENT)
        .set("Accept", "*/*")
        .call()
        .map_err(|e| format!("Yahoo crumb fetch failed: {}", e))?
        .into_string()
        .map_err(|e| format!("Yahoo crumb read failed: {}", e))?
        .trim()
        .to_string();

    if crumb.is_empty() || crumb.len() > 64 {
        return Err("Yahoo crumb invalid".to_string());
    }

    guard.crumb = crumb.clone();
    Ok((guard.agent.clone(), crumb))
}

/// Generic Yahoo Finance GET request with a browser-like User-Agent and crumb.
/// Endpoints that do not require a crumb (like v8/finance/chart) still work
/// because the crumb parameter is ignored when not needed.
pub fn yahoo_request(path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let (agent, crumb) = ensure_session()?;

    let mut url = format!("{}{}", YAHOO_BASE, path);
    let mut first = true;
    for (k, v) in params {
        if first {
            url.push('?');
            first = false;
        } else {
            url.push('&');
        }
        url.push_str(k);
        url.push('=');
        url.push_str(v);
    }
    if !crumb.is_empty() {
        url.push(if first { '?' } else { '&' });
        url.push_str("crumb=");
        url.push_str(&crumb);
    }

    let response = agent
        .get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Yahoo request failed: {}: {}", url, e))?;

    let json: Value = response
        .into_json()
        .map_err(|e| format!("Yahoo JSON parse failed: {}", e))?;

    Ok(json)
}

/// Same as yahoo_request, but does not require a session. Used for endpoints
/// that are known to be crumb-less (e.g. v8/finance/chart).
pub fn yahoo_request_public(path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let mut url = format!("{}{}", YAHOO_BASE, path);
    let mut first = true;
    for (k, v) in params {
        if first {
            url.push('?');
            first = false;
        } else {
            url.push('&');
        }
        url.push_str(k);
        url.push('=');
        url.push_str(v);
    }

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Yahoo request failed: {}: {}", url, e))?;

    let json: Value = response
        .into_json()
        .map_err(|e| format!("Yahoo JSON parse failed: {}", e))?;

    Ok(json)
}

/// Build a DataFrame from a list of named string columns.
pub fn build_string_df(columns: Vec<(&str, Vec<String>)>) -> ArrayRef {
    let fields: Vec<(Arc<Field>, ArrayRef)> = columns
        .into_iter()
        .map(|(name, values)| {
            let array = Arc::new(StringArray::from(values)) as ArrayRef;
            (Arc::new(Field::new(name, DataType::Utf8, false)), array)
        })
        .collect();

    Arc::new(StructArray::from(fields))
}

/// Build a DataFrame from a list of named columns with mixed types.
pub fn build_mixed_df(columns: Vec<(&str, Vec<HayashiValue>)>) -> Result<ArrayRef, String> {
    let mut fields: Vec<(Arc<Field>, ArrayRef)> = Vec::new();

    for (name, values) in columns {
        let is_numeric = matches!(
            values.first(),
            Some(HayashiValue::Float(_)) | Some(HayashiValue::Int(_))
        );
        if is_numeric {
            let nums: Vec<Option<f64>> = values
                .into_iter()
                .map(|v| match v {
                    HayashiValue::Float(f) => Some(f),
                    HayashiValue::Int(i) => Some(i as f64),
                    _ => None,
                })
                .collect();
            let array = Arc::new(Float64Array::from(nums)) as ArrayRef;
            fields.push((Arc::new(Field::new(name, DataType::Float64, true)), array));
        } else {
            let strings: Vec<String> = values
                .into_iter()
                .map(|v| match v {
                    HayashiValue::Str(s) => s,
                    _ => String::new(),
                })
                .collect();
            let array = Arc::new(StringArray::from(strings)) as ArrayRef;
            fields.push((Arc::new(Field::new(name, DataType::Utf8, true)), array));
        }
    }

    Ok(Arc::new(StructArray::from(fields)))
}

/// Extract string field from JSON or empty string.
pub fn json_str(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_string()
}

/// Extract optional JSON string field.
pub fn json_opt_str(value: &Value, key: &str) -> Option<String> {
    value[key].as_str().map(|s| s.to_string())
}

/// Extract JSON number as f64.
pub fn json_f64(value: &Value, key: &str) -> Option<f64> {
    value[key].as_f64()
}

/// Parse a date string (YYYY-MM-DD).
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Extract string option from a Hayashi options dict.
pub fn opt_str(opts: &HashMap<String, HayashiValue>, key: &str) -> Option<String> {
    match opts.get(key) {
        Some(HayashiValue::Str(s)) => Some(s.clone()),
        Some(HayashiValue::Int(i)) => Some(i.to_string()),
        Some(HayashiValue::Float(f)) => Some(f.to_string()),
        _ => None,
    }
}

/// Extract i64 option from a Hayashi options dict.
pub fn opt_i64(opts: &HashMap<String, HayashiValue>, key: &str) -> Option<i64> {
    match opts.get(key) {
        Some(HayashiValue::Int(i)) => Some(*i),
        Some(HayashiValue::Float(f)) => Some(*f as i64),
        Some(HayashiValue::Str(s)) => s.parse::<i64>().ok(),
        _ => None,
    }
}

/// Convert a JSON object into a Hayashi dictionary.
pub fn json_object_to_dict(value: &Value) -> HashMap<String, HayashiValue> {
    let mut dict = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            dict.insert(k.clone(), json_value_to_hayashi(v));
        }
    }
    dict
}

/// Convert a JSON value into a HayashiValue.
pub fn json_value_to_hayashi(value: &Value) -> HayashiValue {
    match value {
        Value::Null => HayashiValue::Str(String::new()),
        Value::Bool(b) => HayashiValue::Str(b.to_string()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                HayashiValue::Int(i)
            } else {
                HayashiValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => HayashiValue::Str(s.clone()),
        Value::Array(arr) => HayashiValue::List(arr.iter().map(json_value_to_hayashi).collect()),
        Value::Object(_) => HayashiValue::Dict(json_object_to_dict(value)),
    }
}

/// Convert a Unix timestamp (ms) to a YYYY-MM-DD string.
pub fn timestamp_ms_to_date(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// Placeholder module for the `set_apikey` function exported by Hayashi plugins.
/// Yahoo Finance does not require an API key, so this is a no-op provided for
/// consistency with the hayfred plugin.
#[hayashi_fn]
pub fn set_apikey(_key: String) {}
