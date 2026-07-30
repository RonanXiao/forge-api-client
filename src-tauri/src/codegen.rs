use crate::auth::apply_auth;
use crate::models::{CodegenInput, KeyValue};

fn merge_headers_query(input: &CodegenInput) -> (Vec<KeyValue>, Vec<KeyValue>, String) {
    let mut headers = input.headers.clone();
    let mut query = input.query.clone();
    let (ah, aq) = apply_auth(&input.auth);
    headers.extend(ah);
    query.extend(aq);

    let mut url = input.url.clone();
    let enabled_q: Vec<_> = query
        .iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .collect();
    if !enabled_q.is_empty() {
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
        if url.contains('?') {
            url.push('&');
            url.push_str(&qs);
        } else {
            url.push('?');
            url.push_str(&qs);
        }
    }
    (headers, query, url)
}

pub fn generate_curl(input: &CodegenInput) -> String {
    let (headers, _, url) = merge_headers_query(input);
    let mut parts = vec![format!("curl -X {} '{}'", input.method.to_uppercase(), url)];
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        parts.push(format!("  -H '{}: {}'", h.key, h.value.replace('\'', "'\\''")));
    }
    let bt = input.body.body_type.to_lowercase();
    if bt != "none" && !input.body.content.is_empty() {
        let escaped = input.body.content.replace('\'', "'\\''");
        parts.push(format!("  --data-raw '{escaped}'"));
    }
    parts.join(" \\\n")
}

pub fn generate_fetch(input: &CodegenInput) -> String {
    let (headers, _, url) = merge_headers_query(input);
    let mut header_obj = String::from("{\n");
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        header_obj.push_str(&format!(
            "    \"{}\": \"{}\",\n",
            h.key,
            h.value.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    header_obj.push_str("  }");

    let method = input.method.to_uppercase();
    let body_js = match input.body.body_type.to_lowercase().as_str() {
        "none" | "" => "undefined".into(),
        "json" => format!(
            "JSON.stringify({})",
            if input.body.content.trim().is_empty() {
                "{}".into()
            } else {
                input.body.content.clone()
            }
        ),
        _ => format!(
            "`{}`",
            input
                .body
                .content
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace('$', "\\$")
        ),
    };

    format!(
        r#"fetch("{url}", {{
  method: "{method}",
  headers: {header_obj},
  body: {body_js}
}}).then(r => r.text()).then(console.log);"#
    )
}

pub fn generate_python(input: &CodegenInput) -> String {
    let (headers, _, url) = merge_headers_query(input);
    let mut header_lines = String::from("{\n");
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        header_lines.push_str(&format!(
            "    \"{}\": \"{}\",\n",
            h.key,
            h.value.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    header_lines.push_str("}");

    let method = input.method.to_lowercase();
    let body_py = match input.body.body_type.to_lowercase().as_str() {
        "none" | "" => "None".into(),
        "json" => format!(
            "'''{}'''",
            input.body.content.replace('\\', "\\\\").replace('\'', "\\'")
        ),
        _ => format!(
            "'''{}'''",
            input.body.content.replace('\\', "\\\\").replace('\'', "\\'")
        ),
    };

    format!(
        r#"import requests

url = "{url}"
headers = {header_lines}
data = {body_py}
response = requests.request("{method}", url, headers=headers, data=data)
print(response.status_code)
print(response.text)"#
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
            body: RequestBody {
                body_type: "json".into(),
                content: r#"{"a":1}"#.into(),
            },
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
}
