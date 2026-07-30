use crate::models::{
    AuthConfig, Collection, CollectionItem, HttpRequest, KeyValue, RequestBody, ScriptBlock,
};
use serde_json::Value;
use uuid::Uuid;

/// Parse a cURL command into an HttpRequest.
/// Supports multi-line `\` continuations, single/double/`$'...'` quotes, common flags.
pub fn parse_curl(input: &str) -> Result<HttpRequest, String> {
    let normalized = normalize_curl_input(input);
    let lower = normalized.to_lowercase();
    if !lower.contains("curl") {
        return Err("Not a cURL command (must contain 'curl')".into());
    }

    let tokens = tokenize_shell(&normalized);
    if tokens.is_empty() {
        return Err("Empty cURL command".into());
    }

    let mut method = String::from("GET");
    let mut method_explicit = false;
    let mut url = String::new();
    let mut headers: Vec<KeyValue> = vec![];
    let mut body = RequestBody {
        body_type: "none".into(),
        content: String::new(),
    };
    let mut auth = AuthConfig {
        auth_type: "none".into(),
        ..Default::default()
    };

    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "curl" | "CURL" | "Curl" => {}
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = tokens[i].to_uppercase();
                    method_explicit = true;
                }
            }
            // -XPOST style
            s if s.starts_with("-X") && s.len() > 2 && !s.starts_with("--") => {
                method = s[2..].to_uppercase();
                method_explicit = true;
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    push_header(&mut headers, &tokens[i]);
                }
            }
            s if s.starts_with("-H") && s.len() > 2 && !s.starts_with("--") => {
                push_header(&mut headers, &s[2..]);
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" | "--data-ascii" => {
                i += 1;
                if i < tokens.len() {
                    set_body_from_data(&mut body, &tokens[i], t);
                    if !method_explicit {
                        method = "POST".into();
                    }
                }
            }
            "--json" => {
                i += 1;
                if i < tokens.len() {
                    body.body_type = "json".into();
                    body.content = tokens[i].clone();
                    if !headers_contain(&headers, "content-type") {
                        headers.push(KeyValue::new("Content-Type", "application/json"));
                    }
                    if !method_explicit {
                        method = "POST".into();
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    let (u, p) = tokens[i]
                        .split_once(':')
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or((tokens[i].clone(), String::new()));
                    auth = AuthConfig {
                        auth_type: "basic".into(),
                        username: u,
                        password: p,
                        ..Default::default()
                    };
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(KeyValue::new("User-Agent", &tokens[i]));
                }
            }
            "-b" | "--cookie" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(KeyValue::new("Cookie", &tokens[i]));
                }
            }
            "-e" | "--referer" => {
                i += 1;
                if i < tokens.len() {
                    headers.push(KeyValue::new("Referer", &tokens[i]));
                }
            }
            // boolean / no-value flags
            "-k" | "--insecure" | "-s" | "--silent" | "-S" | "--show-error" | "-L"
            | "--location" | "-v" | "--verbose" | "-#" | "--progress-bar" | "-i"
            | "--include" | "-I" | "--head" | "-g" | "--globoff" | "-f" | "--fail"
            | "--compressed" | "--http1.1" | "--http2" | "-N" | "--no-buffer" => {
                if t == "-I" || t == "--head" {
                    method = "HEAD".into();
                    method_explicit = true;
                }
            }
            "--url" => {
                i += 1;
                if i < tokens.len() {
                    url = strip_url_quotes(&tokens[i]);
                }
            }
            // skip flags that take one argument we don't model
            "--max-time" | "--connect-timeout" | "-m" | "-o" | "--output" | "-w"
            | "--write-out" | "--proxy" | "-x" | "--cacert" | "--cert" | "--key"
            | "--resolve" | "-E" | "--form" | "-F" => {
                i += 1;
                // -F form: treat as multipart field if possible
                if (t == "--form" || t == "-F") && i < tokens.len() {
                    let field = &tokens[i];
                    if body.body_type == "none" || body.body_type == "multipart" {
                        body.body_type = "multipart".into();
                        if !body.content.is_empty() {
                            body.content.push('\n');
                        }
                        body.content.push_str(field);
                        if !method_explicit {
                            method = "POST".into();
                        }
                    }
                }
            }
            s if s.starts_with('-') => {
                // unknown flag: skip optional value if present
                if i + 1 < tokens.len()
                    && !tokens[i + 1].starts_with('-')
                    && !looks_like_url(&tokens[i + 1])
                {
                    i += 1;
                }
            }
            s if looks_like_url(s) || (url.is_empty() && !s.starts_with('-')) => {
                // First non-flag token that looks like URL, or any leftover URL-ish
                if url.is_empty() || looks_like_url(s) {
                    let candidate = strip_url_quotes(s);
                    if looks_like_url(&candidate) || url.is_empty() {
                        if looks_like_url(&candidate) {
                            url = candidate;
                        } else if url.is_empty() && s != "curl" {
                            // skip bare words that aren't urls
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Second pass for URL if still empty: find any http token
    if url.is_empty() {
        for t in &tokens {
            let c = strip_url_quotes(t);
            if looks_like_url(&c) {
                url = c;
                break;
            }
        }
    }

    if url.is_empty() {
        return Err("No URL found in cURL".into());
    }

    // Infer multipart from content-type header
    if body.body_type == "raw" || body.body_type == "none" {
        if let Some(ct) = headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("content-type"))
        {
            if ct.value.to_lowercase().contains("multipart/form-data") {
                body.body_type = "multipart".into();
            } else if ct.value.to_lowercase().contains("application/x-www-form-urlencoded") {
                body.body_type = "form".into();
            } else if ct.value.to_lowercase().contains("application/json")
                && body.body_type == "raw"
            {
                body.body_type = "json".into();
            }
        }
    }

    let mut query = vec![];
    if let Some((base, qs)) = url.split_once('?') {
        for pair in qs.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair
                .split_once('=')
                .map(|(a, b)| {
                    (
                        urlencoding::decode(a).unwrap_or(a.into()).into_owned(),
                        urlencoding::decode(b).unwrap_or(b.into()).into_owned(),
                    )
                })
                .unwrap_or((pair.to_string(), String::new()));
            query.push(KeyValue::new(k, v));
        }
        url = base.to_string();
    }

    // Ensure trailing empty row helpers on frontend can work
    Ok(HttpRequest {
        id: Uuid::new_v4().to_string(),
        name: "Imported cURL".into(),
        method,
        url,
        headers,
        query,
        body,
        auth,
        config: Default::default(),
        scripts: ScriptBlock::default(),
    })
}

fn normalize_curl_input(input: &str) -> String {
    // Join line continuations: trailing \ before newline
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && (bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r')
        {
            // skip backslash and following newline(s)
            i += 1;
            if i < bytes.len() && bytes[i] == b'\r' {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            // collapse to single space
            if !out.ends_with(' ') {
                out.push(' ');
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out.trim().to_string()
}

fn push_header(headers: &mut Vec<KeyValue>, raw: &str) {
    let raw = raw.trim();
    if let Some((k, v)) = raw.split_once(':') {
        headers.push(KeyValue::new(k.trim(), v.trim()));
    } else if !raw.is_empty() {
        headers.push(KeyValue::new(raw, ""));
    }
}

fn set_body_from_data(body: &mut RequestBody, content: &str, flag: &str) {
    let trimmed = content.trim();
    body.content = content.to_string();
    if flag.contains("urlencode") {
        body.body_type = "form".into();
    } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
        body.body_type = "json".into();
    } else if content.contains("Content-Disposition: form-data")
        || content.contains("WebKitFormBoundary")
        || content.contains("------")
    {
        body.body_type = "multipart".into();
    } else {
        body.body_type = "raw".into();
    }
}

fn headers_contain(headers: &[KeyValue], name: &str) -> bool {
    headers
        .iter()
        .any(|h| h.key.eq_ignore_ascii_case(name))
}

fn strip_url_quotes(s: &str) -> String {
    s.trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string()
}

fn looks_like_url(s: &str) -> bool {
    let s = strip_url_quotes(s);
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("{{")
        || s.starts_with("ws://")
        || s.starts_with("wss://")
}

/// Tokenize shell-like cURL, supporting:
/// - single quotes '...'
/// - double quotes "..." with \" \\
/// - ANSI-C quotes $'...' with \n \r \t \\ \' \" \xHH
fn tokenize_shell(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // ANSI-C quoted string: $'...'
            '$' if chars.peek() == Some(&'\'') => {
                chars.next(); // consume '
                while let Some(ch) = chars.next() {
                    if ch == '\'' {
                        break;
                    }
                    if ch == '\\' {
                        match chars.next() {
                            Some('n') => cur.push('\n'),
                            Some('r') => cur.push('\r'),
                            Some('t') => cur.push('\t'),
                            Some('\\') => cur.push('\\'),
                            Some('\'') => cur.push('\''),
                            Some('"') => cur.push('"'),
                            Some('x') => {
                                let h1 = chars.next().unwrap_or('0');
                                let h2 = chars.next().unwrap_or('0');
                                let hex = format!("{h1}{h2}");
                                if let Ok(v) = u8::from_str_radix(&hex, 16) {
                                    cur.push(v as char);
                                }
                            }
                            Some(other) => cur.push(other),
                            None => {}
                        }
                    } else {
                        cur.push(ch);
                    }
                }
            }
            '\'' => {
                // single-quoted: no escapes except end
                while let Some(ch) = chars.next() {
                    if ch == '\'' {
                        break;
                    }
                    cur.push(ch);
                }
            }
            '"' => {
                while let Some(ch) = chars.next() {
                    if ch == '"' {
                        break;
                    }
                    if ch == '\\' {
                        match chars.next() {
                            Some(n @ ('"' | '\\' | '$' | '`')) => cur.push(n),
                            Some('n') => cur.push('\n'),
                            Some('r') => cur.push('\r'),
                            Some('t') => cur.push('\t'),
                            Some(other) => {
                                cur.push('\\');
                                cur.push(other);
                            }
                            None => {}
                        }
                    } else {
                        cur.push(ch);
                    }
                }
            }
            ' ' | '\t' | '\n' | '\r' => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

/// Import Postman Collection v2.1 JSON into our Collection model.
pub fn import_postman_v21(json: &str) -> Result<Collection, String> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;
    let info = root.get("info").ok_or("Missing info")?;
    let name = info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Postman")
        .to_string();

    let items = root
        .get("item")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(convert_postman_item).collect())
        .unwrap_or_default();

    Ok(Collection {
        id: Uuid::new_v4().to_string(),
        name,
        version: "1.0".into(),
        items,
        scripts: None,
        engine: None,
    })
}

fn convert_postman_item(v: &Value) -> Option<CollectionItem> {
    let name = v.get("name")?.as_str()?.to_string();
    let id = Uuid::new_v4().to_string();

    if v.get("item").and_then(|i| i.as_array()).is_some() && v.get("request").is_none() {
        let children = v
            .get("item")
            .and_then(|i| i.as_array())
            .map(|arr| arr.iter().filter_map(convert_postman_item).collect())
            .unwrap_or_default();
        return Some(CollectionItem {
            id,
            item_type: "folder".into(),
            name,
            children: Some(children),
            request: None,
            scripts: None,
        });
    }

    let req_v = v.get("request")?;
    let method = req_v
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("GET")
        .to_string();
    let url = extract_postman_url(req_v.get("url"));
    let headers = req_v
        .get("header")
        .and_then(|h| h.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|h| {
                    let key = h.get("key")?.as_str()?.to_string();
                    let value = h
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let disabled = h.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                    Some(KeyValue {
                        key,
                        value,
                        enabled: !disabled,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let body = extract_postman_body(req_v.get("body"));
    let auth = extract_postman_auth(req_v.get("auth"));

    Some(CollectionItem {
        id: id.clone(),
        item_type: "request".into(),
        name: name.clone(),
        children: None,
        request: Some(HttpRequest {
            id,
            name,
            method,
            url,
            headers,
            query: vec![],
            body,
            auth,
            config: Default::default(),
            scripts: ScriptBlock::default(),
        }),
        scripts: None,
    })
}

fn extract_postman_url(url_v: Option<&Value>) -> String {
    match url_v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => {
            if let Some(raw) = o.get("raw").and_then(|r| r.as_str()) {
                return raw.to_string();
            }
            let host = o
                .get("host")
                .and_then(|h| h.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                })
                .unwrap_or_default();
            let path = o
                .get("path")
                .and_then(|p| p.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();
            let protocol = o
                .get("protocol")
                .and_then(|p| p.as_str())
                .unwrap_or("https");
            format!("{protocol}://{host}/{path}")
        }
        _ => String::new(),
    }
}

fn extract_postman_body(body_v: Option<&Value>) -> RequestBody {
    let Some(b) = body_v else {
        return RequestBody {
            body_type: "none".into(),
            content: String::new(),
        };
    };
    let mode = b.get("mode").and_then(|m| m.as_str()).unwrap_or("raw");
    match mode {
        "raw" => {
            let content = b
                .get("raw")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let lang = b
                .get("options")
                .and_then(|o| o.get("raw"))
                .and_then(|r| r.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            RequestBody {
                body_type: if lang == "json" || content.trim_start().starts_with('{') {
                    "json".into()
                } else {
                    "raw".into()
                },
                content,
            }
        }
        "urlencoded" => {
            let pairs = b
                .get("urlencoded")
                .and_then(|u| u.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            Some(format!(
                                "{}={}",
                                p.get("key")?.as_str()?,
                                p.get("value").and_then(|v| v.as_str()).unwrap_or("")
                            ))
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            RequestBody {
                body_type: "form".into(),
                content: pairs,
            }
        }
        "formdata" => {
            let pairs = b
                .get("formdata")
                .and_then(|u| u.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            Some(format!(
                                "{}={}",
                                p.get("key")?.as_str()?,
                                p.get("value").and_then(|v| v.as_str()).unwrap_or("")
                            ))
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            RequestBody {
                body_type: "multipart".into(),
                content: pairs,
            }
        }
        _ => RequestBody {
            body_type: "none".into(),
            content: String::new(),
        },
    }
}

fn extract_postman_auth(auth_v: Option<&Value>) -> AuthConfig {
    let Some(a) = auth_v else {
        return AuthConfig {
            auth_type: "none".into(),
            ..Default::default()
        };
    };
    let typ = a.get("type").and_then(|t| t.as_str()).unwrap_or("noauth");
    match typ {
        "bearer" => {
            let token = find_auth_value(a, "bearer", "token").unwrap_or_default();
            AuthConfig {
                auth_type: "bearer".into(),
                token,
                ..Default::default()
            }
        }
        "basic" => AuthConfig {
            auth_type: "basic".into(),
            username: find_auth_value(a, "basic", "username").unwrap_or_default(),
            password: find_auth_value(a, "basic", "password").unwrap_or_default(),
            ..Default::default()
        },
        "apikey" => AuthConfig {
            auth_type: "apikey".into(),
            key: find_auth_value(a, "apikey", "key").unwrap_or_default(),
            value: find_auth_value(a, "apikey", "value").unwrap_or_default(),
            add_to: find_auth_value(a, "apikey", "in").unwrap_or_else(|| "header".into()),
            ..Default::default()
        },
        _ => AuthConfig {
            auth_type: "none".into(),
            ..Default::default()
        },
    }
}

fn find_auth_value(auth: &Value, section: &str, key: &str) -> Option<String> {
    auth.get(section)?
        .as_array()?
        .iter()
        .find(|x| x.get("key").and_then(|k| k.as_str()) == Some(key))
        .and_then(|x| x.get("value"))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_curl() {
        let req = parse_curl(
            r#"curl -X POST 'https://httpbin.org/post?q=1' -H 'Content-Type: application/json' -d '{"x":1}'"#,
        )
        .unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://httpbin.org/post");
        assert_eq!(req.query[0].key, "q");
        assert_eq!(req.body.body_type, "json");
        assert!(req.headers.iter().any(|h| h.key == "Content-Type"));
    }

    #[test]
    fn parse_multiline_backslash() {
        let req = parse_curl(
            r#"curl 'https://example.com/api' \
  -H 'Accept: application/json' \
  -H 'X-Token: abc' \
  --insecure"#,
        )
        .unwrap();
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(req.headers.len(), 2);
        assert!(req.headers.iter().any(|h| h.key == "X-Token"));
    }

    #[test]
    fn parse_ansi_c_quotes_and_multipart() {
        let raw = r#"curl 'http://10.12.105.185:20600/api/test' \
  -H 'Accept: application/json' \
  -H 'Content-Type: multipart/form-data; boundary=----WebKitFormBoundary' \
  --data-raw $'------WebKitFormBoundary\r\nContent-Disposition: form-data; name="pageNo"\r\n\r\n1\r\n------WebKitFormBoundary--\r\n' \
  --insecure"#;
        let req = parse_curl(raw).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "http://10.12.105.185:20600/api/test");
        assert!(
            req.body.body_type == "multipart" || req.body.body_type == "raw",
            "body type={}",
            req.body.body_type
        );
        assert!(req.body.content.contains("pageNo") || req.body.content.contains("form-data"));
        assert!(req.body.content.contains('\n') || req.body.content.contains("pageNo"));
    }

    #[test]
    fn parse_rejects_non_curl() {
        assert!(parse_curl("WebKitFormBoundary only body").is_err());
    }

    #[test]
    fn import_postman_minimal() {
        let json = r#"{
          "info": { "name": "Demo", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
          "item": [
            {
              "name": "Folder A",
              "item": [
                {
                  "name": "Get User",
                  "request": {
                    "method": "GET",
                    "header": [{ "key": "Accept", "value": "application/json" }],
                    "url": { "raw": "https://api.example.com/user" }
                  }
                }
              ]
            }
          ]
        }"#;
        let col = import_postman_v21(json).unwrap();
        assert_eq!(col.name, "Demo");
        assert_eq!(col.items.len(), 1);
        assert_eq!(col.items[0].item_type, "folder");
        let child = &col.items[0].children.as_ref().unwrap()[0];
        assert_eq!(child.name, "Get User");
        assert_eq!(child.request.as_ref().unwrap().method, "GET");
    }
}

#[cfg(test)]
mod user_curl_fixture {
    use super::*;

    #[test]
    fn parse_realistic_browser_copy_curl() {
        // Shape similar to Chrome "Copy as cURL" with ANSI-C body
        let raw = r#"curl 'http://10.12.105.185:20600/api/ebuilder/coms/nlist/getData' \
  -H 'Accept: application/json, text/plain, */*' \
  -H 'Content-Type: multipart/form-data; boundary=----WebKitFormBoundaryLaX8gFpGxFVowHCu' \
  --data-raw $'------WebKitFormBoundaryLaX8gFpGxFVowHCu\r\nContent-Disposition: form-data; name="compId"\r\n\r\nnbacc4ca95d0344159f8963b69c94709c\r\n------WebKitFormBoundaryLaX8gFpGxFVowHCu\r\nContent-Disposition: form-data; name="pageNo"\r\n\r\n1\r\n------WebKitFormBoundaryLaX8gFpGxFVowHCu--\r\n' \
  --insecure"#;
        let req = parse_curl(raw).expect("should parse browser curl");
        assert_eq!(req.method, "POST");
        assert!(req.url.contains("10.12.105.185:20600"));
        assert!(req.headers.iter().any(|h| h.key.eq_ignore_ascii_case("Accept")));
        assert!(req.body.content.contains("compId") || req.body.content.contains("pageNo"));
        assert_ne!(req.url.starts_with("curl"), true);
    }
}
