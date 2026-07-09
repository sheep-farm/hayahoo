#![allow(clippy::not_unsafe_ptr_arg_deref)]
pub mod history;
pub mod quote;
pub mod search;
pub mod utils;

use hayashi_plugin_sdk::hayashi_plugin;

hayashi_plugin!();

#[cfg(test)]
mod tests {
    use super::utils::*;
    use serde_json::json;

    // Testa apenas funções puras (sem rede).

    #[test]
    fn test_timestamp_ms_to_date_known() {
        // 2024-01-15 00:00:00 UTC = 1705276800000 ms
        let result = timestamp_ms_to_date(1_705_276_800_000);
        assert_eq!(result, "2024-01-15");
    }

    #[test]
    fn test_timestamp_ms_to_date_epoch() {
        let result = timestamp_ms_to_date(0);
        assert_eq!(result, "1970-01-01");
    }

    #[test]
    fn test_json_str_present() {
        let v = json!({"symbol": "AAPL", "name": "Apple Inc."});
        assert_eq!(json_str(&v, "symbol"), "AAPL");
        assert_eq!(json_str(&v, "name"), "Apple Inc.");
    }

    #[test]
    fn test_json_str_missing() {
        let v = json!({"symbol": "AAPL"});
        assert_eq!(json_str(&v, "exchange"), "");
    }

    #[test]
    fn test_json_opt_str_present() {
        let v = json!({"currency": "USD"});
        assert_eq!(json_opt_str(&v, "currency"), Some("USD".to_string()));
    }

    #[test]
    fn test_json_opt_str_missing() {
        let v = json!({"currency": "USD"});
        assert_eq!(json_opt_str(&v, "price"), None);
    }

    #[test]
    fn test_json_f64_present() {
        let v = json!({"regularMarketPrice": 189.84});
        let price = json_f64(&v, "regularMarketPrice");
        assert!(price.is_some());
        assert!((price.unwrap() - 189.84).abs() < 1e-6);
    }

    #[test]
    fn test_json_f64_missing() {
        let v = json!({"symbol": "AAPL"});
        assert_eq!(json_f64(&v, "price"), None);
    }

    #[test]
    fn test_parse_date_valid() {
        let d = parse_date("2024-01-15");
        assert!(d.is_some());
        use chrono::Datelike;
        let d = d.unwrap();
        assert_eq!(d.year(), 2024);
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 15);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert_eq!(parse_date("not-a-date"), None);
        assert_eq!(parse_date("2024-13-01"), None);
    }

    #[test]
    fn test_json_object_to_dict_values() {
        use hayashi_plugin_sdk::value::HayashiValue;
        let v = json!({"n": 42, "s": "hello", "f": 3.14});
        let d = json_object_to_dict(&v);
        assert_eq!(d.get("n"), Some(&HayashiValue::Int(42)));
        assert_eq!(d.get("s"), Some(&HayashiValue::Str("hello".to_string())));
        match d.get("f") {
            Some(HayashiValue::Float(f)) => assert!((f - 3.14).abs() < 1e-6),
            _ => panic!("expected Float for 'f'"),
        }
    }
}
