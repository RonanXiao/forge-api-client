use crate::models::{
    AuthConfig, Collection, CollectionItem, HttpRequest, KeyValue, RequestBody, ScriptBlock,
};
use serde_json::Value;
use uuid::Uuid;

/// Parse a cURL command into an HttpRequest.
pub fn parse_curl(input: &str) -> Result<HttpRequest, String> {
    let line = input.trim().replace("\\\n", " ").replace("\\\r\n", " ");
    if !line.to_lowercase().contains("curl") {
        return Err("Not a cURL command".into());
    }

    let tokens = tokenize_shell(&line);
    let mut method = String::from("GET");
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
        let t = &tokens[i];
        match t.as_str() {
            "curl" | "CURL" => {}
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = tokens[i].to_uppercase();
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = tokens[i].split_once(':') {
                        headers.push(KeyValue::new(k.trim(), v.trim()));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
                i += 1;
                if i < tokens.len() {
                    body.body_type = if tokens[i].trim_start().starts_with('{') {
                        "json".into()
                    } else {
                        "raw".into()
                    };
                    body.content = tokens[i].clone();
                    if method == "GET" {
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
            s if s.starts_with('-') => {
                // skip unknown flags; if next looks like value without leading -, skip
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') && !looks_like_url(&tokens[i + 1]) {
                    i += 1;
                }
            }
            s if looks_like_url(s) || (!s.starts_with('-') && url.is_empty() && s != "curl") => {
                url = s.trim_matches(|c| c == '\'' || c == '"').to_string();
            }
            _ => {}
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("No URL found in cURL".into());
    }

    // Split query from URL
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

fn looks_like_url(s: &str) -> bool {
    let s = s.trim_matches(|c| c == '\'' || c == '"');
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("{{")
}

fn tokenize_shell(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' | '\n' if !in_single && !in_double => {
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
    let schema = info
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !schema.is_empty() && !schema.contains("v2.1") && !schema.contains("collection") {
        // still try
    }

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

    // folder if has item array and no request
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
                    let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
            let content = b.get("raw").and_then(|r| r.as_str()).unwrap_or("").to_string();
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
        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
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
