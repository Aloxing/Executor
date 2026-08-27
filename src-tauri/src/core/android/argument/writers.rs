//! Writers that inject resolved argument values into project files.
//! Rust port of `XmlWriter` / `GradleWriter` / `JavaWriter` from
//! `argument_kernel.py`. Python's `re.subn` replaces every match, so all
//! patterns here use `replace_all`.

use regex::{Captures, Regex};
use serde_json::Value;

use super::values::Kind;

/// Dispatches to the writer registered for `file_type`, mirroring the
/// `WRITERS` table (xml / gradle,kts,properties / java,kt,cs).
pub fn write(file_type: &str, text: &str, key: &str, value: &Value, kind: Kind) -> Result<String, String> {
    match file_type {
        "xml" => xml_write(text, key, value, kind),
        "gradle" | "kts" | "properties" => gradle_write(text, key, value, kind),
        "java" | "kt" | "cs" => java_write(text, key, value, kind),
        _ => Err(format!("暂不支持的 file_type: {file_type}")),
    }
}

/// Replaces every match of `pattern`, building each replacement with `f`
/// (closure results are used literally, no `$` expansion). Returns the new
/// text plus the number of replacements, like Python's `re.subn`.
fn sub_all(pattern: &str, text: &str, f: impl Fn(&Captures) -> String) -> (String, usize) {
    let re = match Regex::new(pattern) {
        Ok(re) => re,
        Err(_) => return (text.to_string(), 0),
    };
    let mut count = 0usize;
    let out = re
        .replace_all(text, |caps: &Captures| {
            count += 1;
            f(caps)
        })
        .into_owned();
    (out, count)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_xml(value: &Value, kind: Kind) -> String {
    match kind {
        Kind::Bool => match value {
            Value::Bool(b) => b.to_string(),
            _ => "false".to_string(),
        },
        _ => super::values::display(value),
    }
}

// ----------------------------------------------------------------------
// xml: strings.xml / meta-data / android:xxx attributes
// ----------------------------------------------------------------------

fn xml_write(text: &str, key: &str, value: &Value, kind: Kind) -> Result<String, String> {
    let rendered = render_xml(value, kind);
    let handlers: [fn(&str, &str, &str) -> (String, usize); 3] =
        [xml_string_res, xml_meta_data, xml_attribute];
    for handler in handlers {
        let (new_text, count) = handler(text, key, &rendered);
        if count > 0 {
            return Ok(new_text);
        }
    }
    Err(format!("xml 中未找到可写入的目标: {key}"))
}

fn xml_string_res(text: &str, key: &str, rendered: &str) -> (String, usize) {
    let escaped = escape_xml(rendered);
    let paired = format!(
        r#"(<string\s+name\s*=\s*"{}"[^>]*>)([^<]*)(</string>)"#,
        regex::escape(key)
    );
    let (new_text, count) = sub_all(&paired, text, |caps| {
        format!("{}{}{}", &caps[1], escaped, &caps[3])
    });
    if count > 0 {
        return (new_text, count);
    }
    let empty = format!(r#"<string\s+name\s*=\s*"{}"\s*/>"#, regex::escape(key));
    let replacement = format!("<string name=\"{key}\">{escaped}</string>");
    sub_all(&empty, text, |_| replacement.clone())
}

fn xml_meta_data(text: &str, key: &str, rendered: &str) -> (String, usize) {
    let escaped = escape_xml(rendered);
    let block = match Regex::new(r"(?s)<meta-data\b[^>]*?/?>") {
        Ok(re) => re,
        Err(_) => return (text.to_string(), 0),
    };
    let name_re = match Regex::new(&format!(r#"android:name\s*=\s*"{}""#, regex::escape(key))) {
        Ok(re) => re,
        Err(_) => return (text.to_string(), 0),
    };
    let value_re = match Regex::new(r#"(android:value\s*=\s*")([^"]*)(")"#) {
        Ok(re) => re,
        Err(_) => return (text.to_string(), 0),
    };
    let mut count = 0usize;
    let out = block
        .replace_all(text, |caps: &Captures| {
            let chunk = caps.get(0).unwrap().as_str();
            if !name_re.is_match(chunk) {
                return chunk.to_string();
            }
            // Replace only the first android:value (Python's count=1).
            let updated = if value_re.is_match(chunk) {
                value_re
                    .replace(chunk, |m: &Captures| format!("{}{}{}", &m[1], escaped, &m[3]))
                    .into_owned()
            } else {
                // No android:value attribute: strip the trailing "/>" and append it.
                let trimmed = chunk.trim_end_matches(['/', '>']).trim_end();
                format!("{trimmed}\n            android:value=\"{escaped}\" />")
            };
            count += 1;
            updated
        })
        .into_owned();
    (out, count)
}

fn xml_attribute(text: &str, key: &str, rendered: &str) -> (String, usize) {
    let escaped = escape_xml(rendered);
    let pattern = format!(
        r#"((?:android|tools):{}\s*=\s*")([^"]*)(")"#,
        regex::escape(key)
    );
    sub_all(&pattern, text, |caps| {
        format!("{}{}{}", &caps[1], escaped, &caps[3])
    })
}

// ----------------------------------------------------------------------
// gradle: versionName '1.0.0' / versionCode 1 / applicationId 'x' / key = value
// ----------------------------------------------------------------------

fn gradle_write(text: &str, key: &str, value: &Value, kind: Kind) -> Result<String, String> {
    let rendered = match kind {
        Kind::String => format!("'{}'", super::values::display(value).replace('\'', "\\'")),
        Kind::Bool => match value {
            Value::Bool(b) => b.to_string(),
            _ => "false".to_string(),
        },
        Kind::Int => super::values::display(value),
    };
    let pattern = format!(
        r#"(?m)(^[ \t]*{}[ \t]*(?:=[ \t]*)?)("[^"]*"|'[^']*'|[-+]?\d+(?:\.\d+)?|true|false)"#,
        regex::escape(key)
    );
    let (new_text, count) = sub_all(&pattern, text, |caps| format!("{}{}", &caps[1], rendered));
    if count == 0 {
        return Err(format!("gradle 中未找到键: {key}"));
    }
    Ok(new_text)
}

// ----------------------------------------------------------------------
// java: public static final String KEY = "..."; (type is corrected to the
// value kind, mirroring the Python replacement function)
// ----------------------------------------------------------------------

fn java_write(text: &str, key: &str, value: &Value, kind: Kind) -> Result<String, String> {
    let rendered = match kind {
        Kind::String => {
            // Mirrors Python exactly: double existing "\\" pairs and
            // escape double quotes.
            let s = super::values::display(value);
            format!("\"{}\"", s.replace("\\\\", "\\\\\\\\").replace('"', "\\\""))
        }
        Kind::Bool => match value {
            Value::Bool(b) => b.to_string(),
            _ => "false".to_string(),
        },
        Kind::Int => super::values::display(value),
    };
    let pattern = format!(
        r"(public\s+static\s+final\s*)([A-Za-z_][\w.<>\[\]]*)(\s*\b{}\s*=\s*)([^;]*?)(\s*;)",
        regex::escape(key)
    );
    let expected = match kind {
        Kind::String => "String",
        Kind::Int => "int",
        Kind::Bool => "boolean",
    };
    let (new_text, count) = sub_all(&pattern, text, |caps| {
        let mut declared = caps[2].to_string();
        if kind == Kind::String && declared != "String" {
            declared = expected.to_string();
        } else if kind != Kind::String
            && matches!(declared.as_str(), "String" | "boolean" | "int" | "long")
        {
            declared = expected.to_string();
        }
        format!("{}{}{}{}{}", &caps[1], declared, &caps[3], rendered, &caps[5])
    });
    if count == 0 {
        return Err(format!("java 中未找到常量: {key}"));
    }
    Ok(new_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn xml_string_resource_paired_and_empty() {
        let text = "<resources>\n  <string name=\"app_name\">old</string>\n  <string name=\"empty\" />\n</resources>";
        let out = xml_write(text, "app_name", &json!("我的游戏"), Kind::String).unwrap();
        assert!(out.contains("<string name=\"app_name\">我的游戏</string>"));
        let out = xml_write(text, "empty", &json!("x<y"), Kind::String).unwrap();
        assert!(out.contains("<string name=\"empty\">x&lt;y</string>"));
    }

    #[test]
    fn xml_meta_data_replace_and_append() {
        let text = "<application>\n<meta-data\n    android:name=\"miGameAppId\"\n    android:value=\"old\" />\n<meta-data\n    android:name=\"noValue\" />\n</application>";
        let out = xml_write(text, "miGameAppId", &json!("mi_123"), Kind::String).unwrap();
        assert!(out.contains("android:value=\"mi_123\""));
        // Missing android:value gets appended before the closing "/>".
        let out = xml_write(text, "noValue", &json!("v"), Kind::String).unwrap();
        assert!(out.contains("android:name=\"noValue\" \n            android:value=\"v\" />")
            || out.contains("android:value=\"v\" />"));
    }

    #[test]
    fn xml_attribute() {
        let text = "<activity android:screenOrientation=\"landscape\" />";
        let out = xml_write(text, "screenOrientation", &json!("portrait"), Kind::String).unwrap();
        assert!(out.contains("android:screenOrientation=\"portrait\""));
        assert!(xml_write(text, "missing", &json!("x"), Kind::String).is_err());
    }

    #[test]
    fn gradle_keys() {
        let text = "android {\n    defaultConfig {\n        applicationId 'com.old.pkg'\n        versionCode 1\n        versionName '1.0.0'\n    }\n}";
        let out = gradle_write(text, "versionName", &json!("1.23.2"), Kind::String).unwrap();
        assert!(out.contains("versionName '1.23.2'"));
        let out = gradle_write(text, "versionCode", &json!(123), Kind::Int).unwrap();
        assert!(out.contains("versionCode 123"));
        let out = gradle_write(text, "applicationId", &json!("com.cn.aloxing.xiaomi"), Kind::String).unwrap();
        assert!(out.contains("applicationId 'com.cn.aloxing.xiaomi'"));
        assert!(gradle_write(text, "noSuchKey", &json!("x"), Kind::String).is_err());
    }

    #[test]
    fn java_constants_and_type_correction() {
        let text = "public class Cfg {\n    public static final String Name = \"old\";\n    public static final boolean Show = false;\n    public static final long Code = 1;\n}";
        let out = java_write(text, "Name", &json!("new\"x"), Kind::String).unwrap();
        assert!(out.contains("public static final String Name = \"new\\\"x\";"));
        let out = java_write(text, "Show", &json!(true), Kind::Bool).unwrap();
        assert!(out.contains("public static final boolean Show = true;"));
        // int value on a long declaration corrects the declared type.
        let out = java_write(text, "Code", &json!(123), Kind::Int).unwrap();
        assert!(out.contains("public static final int Code = 123;"));
        assert!(java_write(text, "Missing", &json!("x"), Kind::String).is_err());
    }
}
