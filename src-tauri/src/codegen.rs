//! Generate cURL / fetch / Python from the current request configuration
//! (same body modes as HTTP send: form-data, urlencoded, raw, json, binary).

use crate::auth::apply_auth;
use crate::form_fields::parse_form_fields;
use crate::models::{CodegenInput, KeyValue, RequestBody};

fn shell_single_quote(s: &str) -> String {
    // POSIX: 'foo'\''bar' for embedded single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn merge_headers_query(input: &CodegenInput) -> (Vec<KeyValue>, String) {
    let mut headers = input
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let mut query = input
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();

    let (ah, aq) = apply_auth(&input.auth);
    for h in ah {
        if !headers
            .iter()
            .any(|x| x.key.eq_ignore_ascii_case(&h.key))
        {
            headers.push(h);
        }
    }
    query.extend(aq);

    let mut url = input.url.trim().to_string();
    // Strip existing query from base if we re-append from query list only when
    // query list is non-empty; keep URL as-is if already has ? and query empty.
    let enabled_q: Vec<_> = query
        .iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .collect();
    if !enabled_q.is_empty() {
        // Drop existing query string so we don't double-append
        if let Some((base, _)) = url.split_once('?') {
            url = base.to_string();
        }
        let qs = enabled_q
            .iter()
            .map(|q| {
                format!(
                    "{}={}",
                    urlencoding::encode(&q.key),
                    urlencoding::encode(&q.value)
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        url.push('?');
        url.push_str(&qs);
    }
    (headers, url)
}

fn headers_contain(headers: &[KeyValue], name: &str) -> bool {
    headers
        .iter()
        .any(|h| h.enabled && h.key.eq_ignore_ascii_case(name))
}

fn ensure_header(headers: &mut Vec<KeyValue>, key: &str, value: &str) {
    if !headers_contain(headers, key) {
        headers.push(KeyValue::new(key, value));
    }
}

/// Effective body payload for codegen (mirrors http send_mode).
enum BodyPayload {
    None,
    /// Raw string body + optional Content-Type
    Raw {
        data: String,
        content_type: Option<&'static str>,
    },
    /// application/x-www-form-urlencoded fields
    UrlEncoded(Vec<KeyValue>),
    /// multipart/form-data fields
    FormData(Vec<KeyValue>),
    Binary {
        /// already base64 or raw text from storage
        data: String,
    },
}

fn body_payload(body: &RequestBody) -> BodyPayload {
    let body = body.clone().normalize();
    match body.send_mode() {
        "none" | "" => BodyPayload::None,
        "json" => BodyPayload::Raw {
            data: body.content.clone(),
            content_type: Some("application/json"),
        },
        "raw" => {
            let ct = match body.language.as_deref().map(|s| s.to_lowercase()).as_deref() {
                Some("json") => Some("application/json"),
                Some("javascript") => Some("application/javascript"),
                Some("html") => Some("text/html"),
                Some("xml") => Some("application/xml"),
                Some("text") | None => Some("text/plain"),
                _ => Some("text/plain"),
            };
            BodyPayload::Raw {
                data: body.content.clone(),
                content_type: ct,
            }
        }
        "urlencoded" => {
            let fields = parse_form_fields(&body.content)
                .into_iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .collect::<Vec<_>>();
            if fields.is_empty() && !body.content.trim().is_empty() {
                // Fallback: treat as already-encoded string
                return BodyPayload::Raw {
                    data: body.content.clone(),
                    content_type: Some("application/x-www-form-urlencoded"),
                };
            }
            BodyPayload::UrlEncoded(fields)
        }
        "form-data" => {
            let fields = parse_form_fields(&body.content)
                .into_iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .collect::<Vec<_>>();
            BodyPayload::FormData(fields)
        }
        "binary" => BodyPayload::Binary {
            data: body.content.clone(),
        },
        _ => {
            if body.content.trim().is_empty() {
                BodyPayload::None
            } else {
                BodyPayload::Raw {
                    data: body.content.clone(),
                    content_type: None,
                }
            }
        }
    }
}

fn encode_urlencoded_fields(fields: &[KeyValue]) -> String {
    let mut ser = form_urlencoded::Serializer::new(String::new());
    for f in fields {
        ser.append_pair(&f.key, &f.value);
    }
    ser.finish()
}

pub fn generate_curl(input: &CodegenInput) -> String {
    let (mut headers, url) = merge_headers_query(input);
    let payload = body_payload(&input.body);

    // Auto Content-Type (not for form-data — curl -F sets boundary)
    match &payload {
        BodyPayload::Raw {
            content_type: Some(ct),
            ..
        } => ensure_header(&mut headers, "Content-Type", ct),
        BodyPayload::UrlEncoded(_) => {
            ensure_header(
                &mut headers,
                "Content-Type",
                "application/x-www-form-urlencoded",
            );
        }
        BodyPayload::Binary { .. } => {
            ensure_header(&mut headers, "Content-Type", "application/octet-stream");
        }
        _ => {}
    }

    let mut parts = vec![format!(
        "curl -X {} {}",
        input.method.to_uppercase(),
        shell_single_quote(&url)
    )];

    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        let line = format!("{}: {}", h.key, h.value);
        parts.push(format!("  -H {}", shell_single_quote(&line)));
    }

    match payload {
        BodyPayload::None => {}
        BodyPayload::Raw { data, .. } => {
            if !data.is_empty() {
                parts.push(format!("  --data-raw {}", shell_single_quote(&data)));
            }
        }
        BodyPayload::UrlEncoded(fields) => {
            if !fields.is_empty() {
                let encoded = encode_urlencoded_fields(&fields);
                parts.push(format!("  --data-raw {}", shell_single_quote(&encoded)));
            }
        }
        BodyPayload::FormData(fields) => {
            for f in fields {
                // -F 'key=value' (escape @ for file syntax if value starts with @)
                let val = if f.value.starts_with('@') {
                    format!("\\{}", f.value)
                } else {
                    f.value.clone()
                };
                let pair = format!("{}={}", f.key, val);
                parts.push(format!("  -F {}", shell_single_quote(&pair)));
            }
        }
        BodyPayload::Binary { data } => {
            if !data.is_empty() {
                // Storage is typically base64; emit as --data-binary with raw/base64 note
                parts.push(format!(
                    "  --data-binary {}",
                    shell_single_quote(&data)
                ));
            }
        }
    }

    parts.join(" \\\n")
}

pub fn generate_fetch(input: &CodegenInput) -> String {
    let (mut headers, url) = merge_headers_query(input);
    let payload = body_payload(&input.body);
    let method = input.method.to_uppercase();

    match &payload {
        BodyPayload::Raw {
            content_type: Some(ct),
            ..
        } => ensure_header(&mut headers, "Content-Type", ct),
        BodyPayload::UrlEncoded(_) => {
            ensure_header(
                &mut headers,
                "Content-Type",
                "application/x-www-form-urlencoded",
            );
        }
        BodyPayload::Binary { .. } => {
            ensure_header(&mut headers, "Content-Type", "application/octet-stream");
        }
        // FormData: browser sets Content-Type with boundary — omit manual CT
        BodyPayload::FormData(_) => {
            headers.retain(|h| !h.key.eq_ignore_ascii_case("content-type"));
        }
        _ => {}
    }

    let mut header_obj = String::from("{\n");
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        header_obj.push_str(&format!(
            "    \"{}\": \"{}\",\n",
            h.key,
            h.value.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    header_obj.push_str("  }");

    let body_js = match payload {
        BodyPayload::None => "undefined".into(),
        BodyPayload::Raw { data, content_type } => {
            if content_type == Some("application/json") {
                let trimmed = data.trim();
                if trimmed.is_empty() {
                    "JSON.stringify({})".into()
                } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    format!("JSON.stringify({trimmed})")
                } else {
                    format!(
                        "JSON.stringify({})",
                        serde_json::to_string(&data).unwrap_or_else(|_| "\"\"".into())
                    )
                }
            } else {
                format!(
                    "{}",
                    serde_json::to_string(&data).unwrap_or_else(|_| "\"\"".into())
                )
            }
        }
        BodyPayload::UrlEncoded(fields) => {
            let encoded = encode_urlencoded_fields(&fields);
            serde_json::to_string(&encoded).unwrap_or_else(|_| "\"\"".into())
        }
        BodyPayload::FormData(fields) => {
            let mut lines = vec!["(() => {".to_string(), "  const fd = new FormData();".into()];
            for f in fields {
                lines.push(format!(
                    "  fd.append({}, {});",
                    serde_json::to_string(&f.key).unwrap_or_else(|_| "\"\"".into()),
                    serde_json::to_string(&f.value).unwrap_or_else(|_| "\"\"".into())
                ));
            }
            lines.push("  return fd;".into());
            lines.push("})()".into());
            lines.join("\n")
        }
        BodyPayload::Binary { data } => {
            // assume base64 in storage
            format!(
                "Uint8Array.from(atob({}), c => c.charCodeAt(0))",
                serde_json::to_string(&data).unwrap_or_else(|_| "\"\"".into())
            )
        }
    };

    format!(
        r#"fetch({url}, {{
  method: {method},
  headers: {header_obj},
  body: {body_js}
}}).then(r => r.text()).then(console.log);"#,
        url = serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".into()),
        method = serde_json::to_string(&method).unwrap_or_else(|_| "\"GET\"".into()),
    )
}

pub fn generate_python(input: &CodegenInput) -> String {
    let (mut headers, url) = merge_headers_query(input);
    let payload = body_payload(&input.body);
    let method = input.method.to_lowercase();

    match &payload {
        BodyPayload::Raw {
            content_type: Some(ct),
            ..
        } => ensure_header(&mut headers, "Content-Type", ct),
        BodyPayload::UrlEncoded(_) => {
            // requests sets this for data=dict; still fine to set
            ensure_header(
                &mut headers,
                "Content-Type",
                "application/x-www-form-urlencoded",
            );
        }
        BodyPayload::FormData(_) => {
            // requests multipart: do not set Content-Type manually
            headers.retain(|h| !h.key.eq_ignore_ascii_case("content-type"));
        }
        BodyPayload::Binary { .. } => {
            ensure_header(&mut headers, "Content-Type", "application/octet-stream");
        }
        _ => {}
    }

    let mut header_lines = String::from("{\n");
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        header_lines.push_str(&format!(
            "    {}: {},\n",
            serde_json::to_string(&h.key).unwrap_or_else(|_| "\"\"".into()),
            serde_json::to_string(&h.value).unwrap_or_else(|_| "\"\"".into()),
        ));
    }
    header_lines.push('}');

    let mut needs_json = false;
    let mut needs_b64 = false;

    let (data_arg, files_arg, json_arg) = match payload {
        BodyPayload::None => ("None".into(), "None".into(), "None".into()),
        BodyPayload::Raw { data, content_type } => {
            if content_type == Some("application/json") {
                let trimmed = data.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    needs_json = true;
                    (
                        "None".into(),
                        "None".into(),
                        format!(
                            "json.loads({})",
                            serde_json::to_string(&data).unwrap_or_else(|_| "\"{}\"".into())
                        ),
                    )
                } else {
                    (
                        serde_json::to_string(&data).unwrap_or_else(|_| "None".into()),
                        "None".into(),
                        "None".into(),
                    )
                }
            } else {
                (
                    serde_json::to_string(&data).unwrap_or_else(|_| "None".into()),
                    "None".into(),
                    "None".into(),
                )
            }
        }
        BodyPayload::UrlEncoded(fields) => {
            let mut d = String::from("{\n");
            for f in fields {
                d.push_str(&format!(
                    "    {}: {},\n",
                    serde_json::to_string(&f.key).unwrap_or_else(|_| "\"\"".into()),
                    serde_json::to_string(&f.value).unwrap_or_else(|_| "\"\"".into()),
                ));
            }
            d.push('}');
            (d, "None".into(), "None".into())
        }
        BodyPayload::FormData(fields) => {
            // requests multipart via files= (field_name, (None, value))
            let mut d = String::from("{\n");
            for f in fields {
                d.push_str(&format!(
                    "    {}: (None, {}),\n",
                    serde_json::to_string(&f.key).unwrap_or_else(|_| "\"\"".into()),
                    serde_json::to_string(&f.value).unwrap_or_else(|_| "\"\"".into()),
                ));
            }
            d.push('}');
            ("None".into(), d, "None".into())
        }
        BodyPayload::Binary { data } => {
            needs_b64 = true;
            (
                format!(
                    "base64.b64decode({})",
                    serde_json::to_string(&data).unwrap_or_else(|_| "\"\"".into())
                ),
                "None".into(),
                "None".into(),
            )
        }
    };

    let mut imports = String::from("import requests\n");
    if needs_json {
        imports.push_str("import json\n");
    }
    if needs_b64 {
        imports.push_str("import base64\n");
    }

    format!(
        r#"{imports}
url = {url}
headers = {header_lines}
response = requests.request(
    {method},
    url,
    headers=headers,
    data={data_arg},
    files={files_arg},
    json={json_arg},
)
print(response.status_code)
print(response.text)"#,
        url = serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".into()),
        method = serde_json::to_string(&method).unwrap_or_else(|_| "\"get\"".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuthConfig, RequestBody};

    fn sample() -> CodegenInput {
        CodegenInput {
            method: "POST".into(),
            url: "https://api.example.com/items".into(),
            headers: vec![KeyValue::new("Accept", "application/json")],
            query: vec![KeyValue::new("page", "1")],
            body: RequestBody::with_language("raw", r#"{"a":1}"#, "json"),
            auth: AuthConfig {
                auth_type: "bearer".into(),
                token: "t".into(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn curl_contains_method_auth_body() {
        let out = generate_curl(&sample());
        assert!(out.contains("curl -X POST"));
        assert!(out.contains("Bearer t"));
        assert!(out.contains("page=1"));
        assert!(out.contains(r#"{"a":1}"#));
        assert!(out.contains("application/json"));
    }

    #[test]
    fn curl_urlencoded_from_json_fields() {
        let input = CodegenInput {
            method: "POST".into(),
            url: "https://example.com/api".into(),
            headers: vec![],
            query: vec![],
            body: RequestBody::with(
                "urlencoded",
                r#"[
                  {"key":"filter","value":"{\"a\":1}","enabled":true},
                  {"key":"pageId","value":"123","enabled":true},
                  {"key":"skip","value":"x","enabled":false}
                ]"#,
            ),
            auth: AuthConfig::default(),
        };
        let out = generate_curl(&input);
        assert!(out.contains("--data-raw"), "{out}");
        assert!(out.contains("filter="), "{out}");
        assert!(out.contains("pageId=123") || out.contains("pageId%3D") || out.contains("pageId="), "{out}");
        assert!(!out.contains("skip="), "{out}");
        // Should NOT dump storage JSON array
        assert!(!out.contains(r#"{"key":"filter""#), "{out}");
        assert!(out.contains("application/x-www-form-urlencoded"), "{out}");
    }

    #[test]
    fn curl_form_data_uses_dash_f() {
        let input = CodegenInput {
            method: "POST".into(),
            url: "https://example.com/up".into(),
            headers: vec![],
            query: vec![],
            body: RequestBody::with(
                "form-data",
                r#"[{"key":"compId","value":"abc","enabled":true}]"#,
            ),
            auth: AuthConfig::default(),
        };
        let out = generate_curl(&input);
        assert!(out.contains(" -F "), "{out}");
        assert!(out.contains("compId=abc"), "{out}");
        assert!(!out.contains("--data-raw"), "{out}");
    }

    #[test]
    fn fetch_contains_fetch_call() {
        let out = generate_fetch(&sample());
        assert!(out.contains("fetch("));
        assert!(out.contains("POST"));
        assert!(out.contains("Authorization"));
    }

    #[test]
    fn python_contains_requests() {
        let out = generate_python(&sample());
        assert!(out.contains("import requests"));
        assert!(out.contains("requests.request"));
        assert!(out.contains("post") || out.contains("\"post\""));
    }

    #[test]
    fn python_urlencoded_dict() {
        let input = CodegenInput {
            method: "POST".into(),
            url: "https://example.com".into(),
            headers: vec![],
            query: vec![],
            body: RequestBody::with(
                "urlencoded",
                r#"[{"key":"a","value":"1","enabled":true}]"#,
            ),
            auth: AuthConfig::default(),
        };
        let out = generate_python(&input);
        assert!(out.contains("\"a\": \"1\"") || out.contains("'a'"), "{out}");
    }
}
