//! Value typing, casting and date formatting for the argument kernel.
//! Rust port of the value/type/date section of `argument_kernel.py`.

use chrono::{NaiveDate, NaiveDateTime};
use serde_json::Value;

/// Value kinds inferred from the template value, mirroring Python's
/// `kind_of`: `bool` -> Bool, `null` -> Int, number -> Int, rest String.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    String,
    Int,
    Bool,
}

pub fn kind_of(raw: &Value) -> Kind {
    match raw {
        Value::Bool(_) => Kind::Bool,
        Value::Null => Kind::Int,
        Value::Number(_) => Kind::Int,
        _ => Kind::String,
    }
}

/// Python-style `str(raw)` for non-string JSON values.
pub fn display(raw: &Value) -> String {
    match raw {
        Value::Null => "None".to_string(),
        Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Casts `raw` to the declared `kind`, mirroring Python's `cast_value`.
/// Returns `Err` when a string cannot be parsed as a number (Python
/// raises `ValueError` there, aborting the kernel run).
pub fn cast_value(raw: &Value, kind: Kind) -> Result<Value, String> {
    if raw.is_null() {
        return Ok(match kind {
            Kind::Int => Value::from(0),
            Kind::String => Value::from(""),
            Kind::Bool => Value::from(false),
        });
    }
    match kind {
        Kind::Bool => Ok(match raw {
            Value::Bool(b) => Value::from(*b),
            other => {
                let text = display(other).trim().to_lowercase();
                Value::from(matches!(text.as_str(), "1" | "true" | "yes" | "on"))
            }
        }),
        Kind::Int => {
            let number = match raw {
                Value::Bool(b) => i64::from(*b) as f64,
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                other => display(other)
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("无法将值解析为整型: {other}"))?,
            };
            Ok(Value::from(number as i64))
        }
        Kind::String => Ok(Value::from(display(raw))),
    }
}

// ----------------------------------------------------------------------
// Date formats: Java-style template -> strftime
// ----------------------------------------------------------------------

/// Java-style tokens translated to chrono directives (`SSS` uses `%3f`,
/// the 3-digit fractional, matching millisecond semantics).
const DATE_TOKENS: &[(&str, &str)] = &[
    ("yyyy", "%Y"),
    ("yy", "%y"),
    ("MM", "%m"),
    ("dd", "%d"),
    ("HH", "%H"),
    ("hh", "%I"),
    ("mm", "%M"),
    ("SSS", "%3f"),
    ("ss", "%S"),
];

/// Input patterns tried in order, mirroring `_DATE_INPUTS` plus a few
/// `fromisoformat`-style fallbacks.
const DATETIME_INPUTS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S",
    "%Y/%m/%d %H:%M:%S",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S%.f",
];
const DATE_INPUTS: &[&str] = &["%Y-%m-%d", "%Y/%m/%d", "%Y%m%d"];

pub fn to_strftime(fmt: &str) -> String {
    let mut out = String::new();
    let mut rest = fmt;
    while !rest.is_empty() {
        let mut matched = false;
        for (token, repl) in DATE_TOKENS {
            if let Some(tail) = rest.strip_prefix(token) {
                out.push_str(repl);
                rest = tail;
                matched = true;
                break;
            }
        }
        if !matched {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}

fn parse_datetime(raw: &str) -> Option<NaiveDateTime> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    for pattern in DATETIME_INPUTS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(text, pattern) {
            return Some(dt);
        }
    }
    // Date-only inputs resolve to midnight, same as Python's strptime.
    for pattern in DATE_INPUTS {
        if let Ok(date) = NaiveDate::parse_from_str(text, pattern) {
            return date.and_hms_opt(0, 0, 0);
        }
    }
    None
}

/// Applies a `value_format` to a string value. Non-date content is
/// returned unchanged (mirrors `apply_format`, never breaks the build).
pub fn apply_format(raw: &str, value_format: &str) -> String {
    if value_format.is_empty() {
        return raw.to_string();
    }
    match parse_datetime(raw) {
        Some(dt) => dt.format(&to_strftime(value_format)).to_string(),
        None => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kind_inference() {
        assert_eq!(kind_of(&json!(true)), Kind::Bool);
        assert_eq!(kind_of(&json!(null)), Kind::Int);
        assert_eq!(kind_of(&json!(123)), Kind::Int);
        assert_eq!(kind_of(&json!(1.5)), Kind::Int);
        assert_eq!(kind_of(&json!("")), Kind::String);
        assert_eq!(kind_of(&json!("abc")), Kind::String);
    }

    #[test]
    fn cast_semantics() {
        // null defaults per kind.
        assert_eq!(cast_value(&json!(null), Kind::Int).unwrap(), json!(0));
        assert_eq!(cast_value(&json!(null), Kind::String).unwrap(), json!(""));
        assert_eq!(cast_value(&json!(null), Kind::Bool).unwrap(), json!(false));
        // Bool accepts truthy strings.
        assert_eq!(cast_value(&json!("yes"), Kind::Bool).unwrap(), json!(true));
        assert_eq!(cast_value(&json!("off"), Kind::Bool).unwrap(), json!(false));
        // Int truncates like Python's int(float(x)).
        assert_eq!(cast_value(&json!("12.9"), Kind::Int).unwrap(), json!(12));
        assert_eq!(cast_value(&json!(true), Kind::Int).unwrap(), json!(1));
        assert!(cast_value(&json!("abc"), Kind::Int).is_err());
    }

    #[test]
    fn strftime_translation() {
        assert_eq!(to_strftime("yyyy-MM-dd HH:mm:ss"), "%Y-%m-%d %H:%M:%S");
        assert_eq!(to_strftime("yyyyMMdd"), "%Y%m%d");
    }

    #[test]
    fn format_dates_and_passthrough() {
        assert_eq!(
            apply_format("2026-06-01 19:00:00", "yyyy-MM-dd"),
            "2026-06-01"
        );
        assert_eq!(apply_format("2026/06/01", "yyyy-MM-dd HH:mm:ss"), "2026-06-01 00:00:00");
        // Non-date content passes through unchanged.
        assert_eq!(apply_format("bad-date", "yyyy-MM-dd"), "bad-date");
        assert_eq!(apply_format("", "yyyy-MM-dd"), "");
    }
}
