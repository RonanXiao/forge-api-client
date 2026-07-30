//! Form / multipart field encoding shared by import + HTTP send.
//! Storage format in `body.content` for type form|multipart:
//! - Preferred: JSON array `[{"key":"a","value":"b","enabled":true},...]`
//! - Legacy: newline `key=value` lines

use crate::models::KeyValue;
use serde_json::Value;

/// Parse body content into key/value rows.
pub fn parse_form_fields(content: &str) -> Vec<KeyValue> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    // JSON array of fields
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(arr)) = serde_json::from_str(trimmed) {
            let mut out = Vec::new();
            for item in arr {
                let key = item
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = item
                    .get("value")
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default();
                let enabled = item
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if !key.is_empty() || !value.is_empty() {
                    out.push(KeyValue {
                        key,
                        value,
                        enabled,
                    });
                }
            }
            return out;
        }
    }
    // Raw multipart body → extract fields
    if content.contains("Content-Disposition:") && content.contains("form-data") {
        return parse_multipart_raw(content);
    }
    // Legacy key=value lines (value may not contain unescaped newlines)
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.push(KeyValue::new(k.trim(), v.trim()));
        }
    }
    out
}

/// Serialize fields to JSON storage format.
pub fn fields_to_json(fields: &[KeyValue]) -> String {
    let arr: Vec<Value> = fields
        .iter()
        .filter(|f| !f.key.is_empty() || !f.value.is_empty())
        .map(|f| {
            serde_json::json!({
                "key": f.key,
                "value": f.value,
                "enabled": f.enabled,
            })
        })
        .collect();
    serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".into())
}

/// Parse a raw multipart/form-data body (with boundaries) into fields.
/// Values are decoded as UTF-8 (lossy for binary parts).
pub fn parse_multipart_raw(content: &str) -> Vec<KeyValue> {
    // Normalize line endings
    let content = content.replace("\r\n", "\n").replace('\r', "\n");

    // Detect boundary from first line starting with --
    let boundary = content.lines().find_map(|line| {
        let t = line.trim();
        if t.starts_with("--") && t.len() > 2 && !t.ends_with("--") {
            Some(t.trim_start_matches('-').to_string())
        } else if t.starts_with("------") {
            // ------WebKit...
            Some(t.trim_start_matches('-').to_string())
        } else {
            None
        }
    });

    let mut fields = Vec::new();

    if let Some(ref b) = boundary {
        // Split on --boundary
        let delim = format!("--{b}");
        for part in content.split(&delim) {
            let part = part.trim();
            if part.is_empty() || part == "--" || part.starts_with("--") {
                continue;
            }
            // Strip trailing --
            let part = part.trim_end_matches('-').trim();
            if let Some(kv) = parse_multipart_part(part) {
                fields.push(kv);
            }
        }
    } else {
        // Fallback: look for name="..." patterns
        for chunk in content.split("Content-Disposition:") {
            if let Some(kv) = parse_multipart_part(&format!("Content-Disposition:{chunk}")) {
                fields.push(kv);
            }
        }
    }

    fields
}

fn parse_multipart_part(part: &str) -> Option<KeyValue> {
    let part = part.trim();
    if part.is_empty() {
        return None;
    }
    // Headers then blank line then body
    let (headers, body) = if let Some(idx) = part.find("\n\n") {
        (&part[..idx], &part[idx + 2..])
    } else if let Some(idx) = part.find("\r\n\r\n") {
        (&part[..idx], &part[idx + 4..])
    } else {
        // single block: try name= only
        (part, "")
    };

    let mut name = None;
    let mut is_file = false;
    for line in headers.lines() {
        let line = line.trim();
        if line.to_ascii_lowercase().starts_with("content-disposition:") {
            // name="foo" or name=foo
            if let Some(n) = extract_disposition_param(line, "name") {
                name = Some(n);
            }
            if extract_disposition_param(line, "filename").is_some() {
                is_file = true;
            }
        }
    }
    let name = name?;
    // Value: trim one trailing newline from body
    let mut value = body.to_string();
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    // Binary / non-utf8: if heavily non-printable, mark as base64 note
    if is_file || is_mostly_binary(&value) {
        // Keep UTF-8 lossy text if possible; for binary store as base64 with prefix
        if is_mostly_binary(&value) {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(value.as_bytes());
            value = format!("[binary:base64]{b64}");
        }
    }

    Some(KeyValue::new(name, value))
}

fn extract_disposition_param(header: &str, param: &str) -> Option<String> {
    // name="..." or name=...
    let key = format!("{param}=");
    let lower = header.to_ascii_lowercase();
    let pos = lower.find(&key)?;
    let rest = &header[pos + key.len()..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let end = rest
            .find(|c: char| c == ';' || c == ' ' || c == '\n' || c == '\r')
            .unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

fn is_mostly_binary(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let non_print = s
        .chars()
        .filter(|c| {
            let u = *c as u32;
            // allow tab newline cr
            if *c == '\n' || *c == '\r' || *c == '\t' {
                return false;
            }
            u < 9 || (u >= 0x0E && u < 0x20) || (*c == '\u{FFFD}')
        })
        .count();
    // high ratio of replacement/control chars
    non_print * 4 > s.chars().count()
}

/// Convert raw multipart body content into JSON field storage if applicable.
pub fn normalize_body_content(body_type: &str, content: &str) -> (String, String) {
    let bt = body_type.to_lowercase();
    if bt == "multipart" || bt == "form" {
        let fields = parse_form_fields(content);
        if !fields.is_empty() {
            return (bt, fields_to_json(&fields));
        }
    }
    (body_type.to_string(), content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_webkit_multipart() {
        let raw = "------WebKitFormBoundaryLaX8gFpGxFVowHCu\r\n\
Content-Disposition: form-data; name=\"compId\"\r\n\
\r\n\
nbacc4ca95d\r\n\
------WebKitFormBoundaryLaX8gFpGxFVowHCu\r\n\
Content-Disposition: form-data; name=\"pageNo\"\r\n\
\r\n\
1\r\n\
------WebKitFormBoundaryLaX8gFpGxFVowHCu\r\n\
Content-Disposition: form-data; name=\"beginDatetime\"\r\n\
\r\n\
2026-07-31 12:00\r\n\
------WebKitFormBoundaryLaX8gFpGxFVowHCu--\r\n";
        let fields = parse_multipart_raw(raw);
        assert!(fields.iter().any(|f| f.key == "compId" && f.value == "nbacc4ca95d"));
        assert!(fields.iter().any(|f| f.key == "pageNo" && f.value == "1"));
        assert!(fields
            .iter()
            .any(|f| f.key == "beginDatetime" && f.value.contains("2026-07-31")));
        // round-trip json
        let json = fields_to_json(&fields);
        let again = parse_form_fields(&json);
        assert_eq!(again.len(), fields.len());
    }
}
