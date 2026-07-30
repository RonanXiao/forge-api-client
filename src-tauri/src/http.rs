use crate::auth::apply_auth;
use crate::cookies::{cookie_header, ingest_set_cookie};
use crate::env_interp::{interpolate, interpolate_kv_list};
use crate::models::{
    CookieEntry, HttpResponse, KeyValue, ProxyConfig, RequestBody, SendRequestInput,
};
use base64::Engine;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method, Proxy,
};
use std::str::FromStr;
use std::time::{Duration, Instant};
use url::Url;

/// Pure assembly of final URL, headers, query for testing without network.
pub fn assemble_request(
    input: &SendRequestInput,
) -> Result<(String, String, Vec<KeyValue>, Vec<KeyValue>, RequestBody), String> {
    let vars = &input.variables;
    let method = input.method.to_uppercase();
    let mut url = interpolate(&input.url, vars);
    let mut headers = interpolate_kv_list(&input.headers, vars);
    let mut query = interpolate_kv_list(&input.query, vars);
    let body = RequestBody {
        body_type: input.body.body_type.clone(),
        content: interpolate(&input.body.content, vars),
        language: input.body.language.clone(),
    }
    .normalize();

    let mut auth = input.auth.clone();
    auth.token = interpolate(&auth.token, vars);
    auth.username = interpolate(&auth.username, vars);
    auth.password = interpolate(&auth.password, vars);
    auth.key = interpolate(&auth.key, vars);
    auth.value = interpolate(&auth.value, vars);

    let (ah, aq) = apply_auth(&auth);
    for h in ah {
        if !headers
            .iter()
            .any(|x| x.enabled && x.key.eq_ignore_ascii_case(&h.key))
        {
            headers.push(h);
        }
    }
    query.extend(aq);

    // Attach query to URL string for display/assembly check
    let enabled_q: Vec<_> = query
        .iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .collect();
    if !enabled_q.is_empty() {
        let mut u = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;
        {
            let mut pairs = u.query_pairs_mut();
            for q in &enabled_q {
                pairs.append_pair(&q.key, &q.value);
            }
        }
        url = u.to_string();
    }

    Ok((method, url, headers, query, body))
}

pub async fn send_request(
    input: SendRequestInput,
    cookie_jar: Option<&mut Vec<CookieEntry>>,
) -> Result<HttpResponse, String> {
    let (method_s, url_s, mut headers, query, body) = assemble_request(&input)?;

    let timeout_ms = input
        .config
        .timeout_ms
        .or(input.timeout_ms)
        .unwrap_or(30_000);
    let follow = input.config.follow_redirects.unwrap_or(true);
    let max_redirects = input.config.max_redirects.unwrap_or(10);

    let redirect_policy = if follow {
        reqwest::redirect::Policy::limited(max_redirects as usize)
    } else {
        reqwest::redirect::Policy::none()
    };

    let mut builder = Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .redirect(redirect_policy)
        .danger_accept_invalid_certs(false);

    if let Some(proxy_cfg) = input.proxy.as_ref() {
        apply_proxy(&mut builder, proxy_cfg)?;
    }

    let client = builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let method = Method::from_str(&method_s)
        .map_err(|e| format!("Invalid HTTP method '{method_s}': {e}"))?;

    let url = Url::parse(&url_s).map_err(|e| format!("Invalid URL: {e}"))?;

    // Cookie header
    if !input.skip_cookies {
        if let Some(jar) = cookie_jar.as_ref() {
            if let Some(ch) = cookie_header(jar, &url_s) {
                if !headers
                    .iter()
                    .any(|h| h.enabled && h.key.eq_ignore_ascii_case("cookie"))
                {
                    headers.push(KeyValue::new("Cookie", ch));
                }
            }
        }
    }

    let mut header_map = HeaderMap::new();
    let mut out_headers: Vec<(String, String)> = Vec::new();
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        let name = HeaderName::from_bytes(h.key.as_bytes())
            .map_err(|e| format!("Invalid header name '{}': {e}", h.key))?;
        let value = HeaderValue::from_str(&h.value)
            .map_err(|e| format!("Invalid header value for '{}': {e}", h.key))?;
        header_map.append(name, value);
        out_headers.push((h.key.clone(), h.value.clone()));
    }

    // If URL from assemble already has query, use as-is. Also re-apply query list
    // in case URL parse stripped nothing - assemble already included them.
    let _ = query; // used in assemble

    let scheme = url.scheme().to_string();
    let host = url.host_str().unwrap_or("").to_string();
    let port = url
        .port_or_known_default()
        .map(|p| p.to_string())
        .unwrap_or_default();
    let path_q = {
        let p = url.path();
        match url.query() {
            Some(q) => format!("{p}?{q}"),
            None => p.to_string(),
        }
    };

    let mut request_builder = client.request(method.clone(), url.clone()).headers(header_map);

    // Effective body metadata for verbose (curl -v style)
    let mut body_note = String::from("none");
    let mut req_body_len: Option<usize> = None;
    let body_mode = body.send_mode().to_string();

    // Postman modes (+ legacy aliases via send_mode)
    match body_mode.as_str() {
        "json" => {
            if !headers_contain(&headers, "content-type") {
                request_builder = request_builder.header("Content-Type", "application/json");
                out_headers.push(("Content-Type".into(), "application/json".into()));
            }
            req_body_len = Some(body.content.len());
            body_note = format!("json ({} bytes)", body.content.len());
            request_builder = request_builder.body(body.content.clone());
        }
        "urlencoded" => {
            if !headers_contain(&headers, "content-type") {
                request_builder = request_builder
                    .header("Content-Type", "application/x-www-form-urlencoded");
                out_headers.push((
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                ));
            }
            let fields = crate::form_fields::parse_form_fields(&body.content);
            let encoded = if !fields.is_empty() {
                let mut ser = form_urlencoded::Serializer::new(String::new());
                for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                    ser.append_pair(&f.key, &f.value);
                }
                ser.finish()
            } else {
                encode_form_body(&body.content)
            };
            req_body_len = Some(encoded.len());
            body_note = format!("x-www-form-urlencoded ({} bytes)", encoded.len());
            request_builder = request_builder.body(encoded);
        }
        "raw" => {
            if !headers_contain(&headers, "content-type") {
                if let Some(ct) = raw_content_type(body.language.as_deref()) {
                    request_builder = request_builder.header("Content-Type", ct);
                    out_headers.push(("Content-Type".into(), ct.into()));
                }
            }
            req_body_len = Some(body.content.len());
            let lang = body.language.as_deref().unwrap_or("text");
            body_note = format!("raw/{lang} ({} bytes)", body.content.len());
            request_builder = request_builder.body(body.content.clone());
        }
        "binary" => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(body.content.trim())
                .unwrap_or_else(|_| body.content.as_bytes().to_vec());
            req_body_len = Some(bytes.len());
            body_note = format!("binary ({} bytes)", bytes.len());
            request_builder = request_builder.body(bytes);
        }
        "form-data" => {
            let fields = crate::form_fields::parse_form_fields(&body.content);
            let mut form = reqwest::multipart::Form::new();
            let mut field_count = 0usize;
            for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                let value = if let Some(b64) = f.value.strip_prefix("[binary:base64]") {
                    // decode binary field if we stored it that way
                    base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .ok()
                        .and_then(|b| String::from_utf8(b).ok())
                        .unwrap_or_else(|| f.value.clone())
                } else {
                    f.value.clone()
                };
                form = form.text(f.key.clone(), value);
                field_count += 1;
            }
            // Do not set Content-Type manually — reqwest generates boundary
            out_headers.push((
                "Content-Type".into(),
                "multipart/form-data; boundary=<auto>".into(),
            ));
            body_note = format!("multipart/form-data ({field_count} fields, boundary auto)");
            request_builder = request_builder.multipart(form);
        }
        _ => {}
    }

    // Ensure Host is shown in verbose even if not explicitly set
    if !out_headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("host"))
        && !host.is_empty()
    {
        let host_line = if port.is_empty() {
            host.clone()
        } else if (scheme == "https" && port == "443") || (scheme == "http" && port == "80") {
            host.clone()
        } else {
            format!("{host}:{port}")
        };
        out_headers.insert(0, ("Host".into(), host_line));
    }

    let mut verbose = String::new();
    verbose.push_str(&format!("* URL: {url_s}\n"));
    if !host.is_empty() {
        verbose.push_str(&format!("* Host {host} port {port} was resolved.\n"));
        verbose.push_str(&format!("* Trying {host}:{port}…\n"));
        verbose.push_str(&format!(
            "* Connected to {host} ({scheme}) port {port}\n"
        ));
    }
    if let Some(ref proxy) = input.proxy {
        verbose.push_str(&format!("* Proxy mode: {}\n", proxy.mode));
    }
    verbose.push_str(&format!(
        "* Timeout: {} ms · Follow redirects: {} (max {})\n",
        timeout_ms, follow, max_redirects
    ));

    let start = Instant::now();
    let response = match request_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            verbose.push_str(&format!(
                "> {} {} HTTP/1.1\n",
                method_s,
                if path_q.is_empty() { "/" } else { &path_q }
            ));
            for (k, v) in &out_headers {
                verbose.push_str(&format!("> {k}: {v}\n"));
            }
            verbose.push_str(">\n");
            verbose.push_str(&format!("* Request body: {body_note}\n"));
            verbose.push_str("* Sending request to server\n");
            verbose.push_str(&format!("* Request failed after {elapsed} ms\n"));
            verbose.push_str(&format!("* Error: {e}\n"));
            return Err(format!("Request failed: {e}\n\n--- verbose ---\n{verbose}"));
        }
    };
    let duration_ms = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();
    let http_ver = version_label(response.version());
    let remote = response
        .remote_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| host.clone());
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let set_cookies: Vec<String> = response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
        .collect();

    let resp_headers: Vec<KeyValue> = response
        .headers()
        .iter()
        .map(|(k, v)| KeyValue {
            key: k.to_string(),
            value: v.to_str().unwrap_or("").to_string(),
            enabled: true,
        })
        .collect();

    // Request block (curl / Apifox console style)
    verbose.push_str(&format!(
        "> {} {} {}\n",
        method_s,
        if path_q.is_empty() { "/" } else { &path_q },
        http_ver
    ));
    for (k, v) in &out_headers {
        verbose.push_str(&format!("> {k}: {v}\n"));
    }
    verbose.push_str(">\n");
    verbose.push_str(&format!("* Request body: {body_note}\n"));
    if let Some(n) = req_body_len {
        verbose.push_str(&format!("* upload completely sent off: {n} bytes\n"));
    }
    if final_url != url_s {
        verbose.push_str(&format!("* Redirected to: {final_url}\n"));
    }
    if !remote.is_empty() {
        verbose.push_str(&format!("* Remote address: {remote}\n"));
    }

    // Gray mid-status tips (like Apifox Network console)
    verbose.push_str("* Sending request to server\n");

    // Response status + headers
    verbose.push_str(&format!("< {} {} {}\n", http_ver, status, status_text));
    for h in &resp_headers {
        verbose.push_str(&format!("< {}: {}\n", h.key, h.value));
    }
    verbose.push_str("<\n");

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;
    let body_size = bytes.len() as u64;
    let body_text = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    };

    verbose.push_str(&format!(
        "* Connection #0 to host {host} left intact\n"
    ));
    verbose.push_str(&format!("* Total time: {duration_ms} ms\n"));
    if let Some(ref ct) = content_type {
        verbose.push_str(&format!("* Content-Type: {ct}\n"));
    }
    // Trailing size tip — same phrasing as Apifox
    verbose.push_str(&format!("* [{} received]\n", format_size_label(body_size)));

    if !input.skip_cookies {
        if let Some(jar) = cookie_jar {
            ingest_set_cookie(jar, &url_s, &set_cookies);
        }
    }

    Ok(HttpResponse {
        status,
        status_text,
        headers: resp_headers,
        body: body_text,
        body_size,
        duration_ms,
        content_type,
        verbose,
    })
}

fn version_label(v: reqwest::Version) -> &'static str {
    match v {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2",
        reqwest::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/?",
    }
}

fn format_size_label(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn apply_proxy(builder: &mut reqwest::ClientBuilder, cfg: &ProxyConfig) -> Result<(), String> {
    match cfg.mode.to_lowercase().as_str() {
        "none" => {
            // rebuild without system proxy by using no_proxy
            let taken = std::mem::replace(builder, Client::builder());
            *builder = taken.no_proxy();
            Ok(())
        }
        "manual" => {
            let mut taken = std::mem::replace(builder, Client::builder());
            if let Some(ref http) = cfg.http {
                let p = Proxy::http(http).map_err(|e| format!("Invalid HTTP proxy: {e}"))?;
                taken = taken.proxy(p);
            }
            if let Some(ref https) = cfg.https {
                let p = Proxy::https(https).map_err(|e| format!("Invalid HTTPS proxy: {e}"))?;
                taken = taken.proxy(p);
            }
            if let Some(ref socks) = cfg.socks {
                let p = Proxy::all(socks).map_err(|e| format!("Invalid SOCKS proxy: {e}"))?;
                taken = taken.proxy(p);
            }
            *builder = taken;
            Ok(())
        }
        _ => {
            // system - default reqwest with system-proxy feature
            Ok(())
        }
    }
}

fn encode_form_body(content: &str) -> String {
    // Accept already-encoded or key=value lines
    if content.contains('&') && !content.contains('\n') {
        return content.to_string();
    }
    let mut ser = form_urlencoded::Serializer::new(String::new());
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            ser.append_pair(k.trim(), v.trim());
        } else if let Some((k, v)) = line.split_once(':') {
            ser.append_pair(k.trim(), v.trim());
        }
    }
    ser.finish()
}

fn headers_contain(headers: &[KeyValue], name: &str) -> bool {
    headers
        .iter()
        .any(|h| h.enabled && h.key.eq_ignore_ascii_case(name))
}

fn raw_content_type(language: Option<&str>) -> Option<&'static str> {
    match language.map(|s| s.to_lowercase()).as_deref() {
        Some("json") => Some("application/json"),
        Some("javascript") => Some("application/javascript"),
        Some("html") => Some("text/html"),
        Some("xml") => Some("application/xml"),
        Some("text") | None => Some("text/plain"),
        _ => Some("text/plain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuthConfig, RequestConfig};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn spawn_echo_server() -> (String, Arc<Mutex<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        let cap2 = captured.clone();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                *cap2.lock().unwrap() = req.clone();
                let body = r#"{"ok":true,"echo":1}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nSet-Cookie: session=xyz; Path=/\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        (format!("http://{addr}"), captured)
    }

    #[test]
    fn assemble_applies_vars_and_bearer() {
        let mut vars = HashMap::new();
        vars.insert("base".into(), "http://localhost:9".into());
        let input = SendRequestInput {
            method: "GET".into(),
            url: "{{base}}/path".into(),
            headers: vec![],
            query: vec![KeyValue::new("q", "1")],
            body: RequestBody {
                body_type: "none".into(),
                content: String::new(),
                language: None,
            },
            auth: AuthConfig {
                auth_type: "bearer".into(),
                token: "tok".into(),
                ..Default::default()
            },
            config: RequestConfig::default(),
            timeout_ms: None,
            variables: vars,
            proxy: None,
            skip_cookies: true,
        };
        let (m, url, headers, _, _) = assemble_request(&input).unwrap();
        assert_eq!(m, "GET");
        assert!(url.starts_with("http://localhost:9/path"));
        assert!(url.contains("q=1"));
        assert!(headers.iter().any(|h| h.value == "Bearer tok"));
    }

    #[tokio::test]
    async fn send_get_json_and_cookies() {
        let (base, captured) = spawn_echo_server();
        let mut jar = vec![];
        let input = SendRequestInput {
            method: "GET".into(),
            url: format!("{base}/hello"),
            headers: vec![KeyValue::new("X-Test", "1")],
            query: vec![],
            body: RequestBody {
                body_type: "none".into(),
                content: String::new(),
                language: None,
            },
            auth: AuthConfig {
                auth_type: "none".into(),
                ..Default::default()
            },
            config: RequestConfig {
                timeout_ms: Some(5_000),
                follow_redirects: Some(true),
                max_redirects: Some(5),
            },
            timeout_ms: None,
            variables: HashMap::new(),
            proxy: Some(ProxyConfig {
                mode: "none".into(),
                ..Default::default()
            }),
            skip_cookies: false,
        };
        let resp = send_request(input, Some(&mut jar)).await.unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("ok"));
        assert!(resp.duration_ms < 30_000);
        assert!(resp.body_size > 0);
        assert_eq!(jar.len(), 1);
        assert_eq!(jar[0].name, "session");
        let cap = captured.lock().unwrap().clone();
        assert!(cap.contains("X-Test: 1") || cap.contains("x-test: 1"));
    }

    #[tokio::test]
    async fn send_post_json_body() {
        let (base, captured) = spawn_echo_server();
        let input = SendRequestInput {
            method: "POST".into(),
            url: base,
            headers: vec![],
            query: vec![],
            body: RequestBody::with_language("raw", r#"{"hello":"world"}"#, "json"),
            auth: AuthConfig {
                auth_type: "none".into(),
                ..Default::default()
            },
            config: Default::default(),
            timeout_ms: Some(5_000),
            variables: HashMap::new(),
            proxy: Some(ProxyConfig {
                mode: "none".into(),
                ..Default::default()
            }),
            skip_cookies: true,
        };
        let resp = send_request(input, None).await.unwrap();
        assert_eq!(resp.status, 200);
        let cap = captured.lock().unwrap().clone();
        assert!(cap.contains(r#"{"hello":"world"}"#));
        assert!(cap.contains("application/json"));
    }
}
