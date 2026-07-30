mod js_engine;
mod rhai_engine;

use crate::models::{
    AssertionResult, ExecuteScriptsInput, HttpResponse, MutableRequest, ScriptPermissions,
    ScriptRunResult,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct ScriptContext {
    pub request: MutableRequest,
    pub response: Option<HttpResponse>,
    pub variables: HashMap<String, String>,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
    pub assertions: Vec<AssertionResult>,
    pub permissions: ScriptPermissions,
    pub fs_root: PathBuf,
}

impl ScriptContext {
    pub fn from_input(input: &ExecuteScriptsInput) -> Self {
        Self {
            request: input.request.clone(),
            response: input.response.clone(),
            variables: input.variables.clone(),
            logs: vec![],
            errors: vec![],
            assertions: vec![],
            permissions: input.permissions.clone(),
            fs_root: PathBuf::from(&input.fs_root),
        }
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    pub fn set_var(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_var(&self, key: &str) -> String {
        self.variables.get(key).cloned().unwrap_or_default()
    }

    pub fn assert_status(&mut self, expected: i64) {
        let actual = self.response.as_ref().map(|r| r.status as i64).unwrap_or(-1);
        let passed = actual == expected;
        self.assertions.push(AssertionResult {
            name: "status".into(),
            passed,
            message: format!("expected status {expected}, got {actual}"),
        });
    }

    pub fn assert_duration_lt(&mut self, max_ms: i64) {
        let actual = self.response.as_ref().map(|r| r.duration_ms as i64).unwrap_or(-1);
        let passed = actual >= 0 && actual < max_ms;
        self.assertions.push(AssertionResult {
            name: "duration".into(),
            passed,
            message: format!("expected duration < {max_ms}ms, got {actual}ms"),
        });
    }

    pub fn assert_body_field(&mut self, path: &str, expected: &str) {
        let body = self
            .response
            .as_ref()
            .map(|r| r.body.as_str())
            .unwrap_or("");
        let actual = json_path_string(body, path);
        let passed = actual.as_deref() == Some(expected);
        self.assertions.push(AssertionResult {
            name: format!("body.{path}"),
            passed,
            message: format!(
                "expected body field '{path}' == '{expected}', got '{}'",
                actual.unwrap_or_else(|| "<missing>".into())
            ),
        });
    }

    pub fn fs_read(&self, rel: &str) -> Result<String, String> {
        if !self.permissions.allow_fs {
            return Err("Filesystem access denied".into());
        }
        let path = safe_path(&self.fs_root, rel)?;
        std::fs::read_to_string(&path).map_err(|e| format!("fs read: {e}"))
    }

    pub fn fs_write(&self, rel: &str, content: &str) -> Result<(), String> {
        if !self.permissions.allow_fs {
            return Err("Filesystem access denied".into());
        }
        let path = safe_path(&self.fs_root, rel)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("fs mkdir: {e}"))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("fs write: {e}"))
    }

    pub fn fs_append(&self, rel: &str, content: &str) -> Result<(), String> {
        if !self.permissions.allow_fs {
            return Err("Filesystem access denied".into());
        }
        let path = safe_path(&self.fs_root, rel)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("fs mkdir: {e}"))?;
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("fs append open: {e}"))?;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("fs append: {e}"))
    }

    pub fn into_result(self) -> ScriptRunResult {
        ScriptRunResult {
            logs: self.logs,
            errors: self.errors,
            assertions: self.assertions,
            variables: self.variables,
            request: Some(self.request),
        }
    }
}

fn safe_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let joined = root.join(rel);
    let canon_root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    // For non-existing files, canonicalize parent
    let target = if joined.exists() {
        joined.canonicalize().map_err(|e| format!("path: {e}"))?
    } else {
        let parent = joined.parent().unwrap_or(root);
        let file = joined.file_name().ok_or("invalid path")?;
        let cp = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        cp.join(file)
    };
    if !target.starts_with(&canon_root) {
        return Err("Path escapes fs root".into());
    }
    Ok(target)
}

fn json_path_string(body: &str, path: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let mut cur = &v;
    for part in path.split('.').filter(|p| !p.is_empty()) {
        cur = cur.get(part)?;
    }
    match cur {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => Some("null".into()),
        other => Some(other.to_string()),
    }
}

pub fn tools_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn tools_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn tools_date_format(fmt: &str) -> String {
    let now = chrono::Utc::now();
    // Support a few common tokens
    let fmt = fmt
        .replace("YYYY", "%Y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S");
    now.format(&fmt).to_string()
}

pub fn tools_base64_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

pub fn tools_base64_decode(s: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| format!("base64 decode: {e}"))?;
    String::from_utf8(bytes).map_err(|e| format!("utf8: {e}"))
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn tools_md5(s: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(s.as_bytes());
    hex_encode(hasher.finalize())
}

pub fn tools_sha256(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex_encode(hasher.finalize())
}

pub fn tools_json_parse(s: &str) -> Result<Value, String> {
    serde_json::from_str(s).map_err(|e| format!("json parse: {e}"))
}

pub fn tools_json_stringify(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
}

/// Run scripts for the given phase using the selected engine.
pub fn execute_scripts(input: ExecuteScriptsInput) -> ScriptRunResult {
    let engine = input.engine.to_lowercase();
    let phase = input.phase.to_lowercase();
    let timeout_ms = input.permissions.timeout_ms.max(1);
    let ctx = Arc::new(Mutex::new(ScriptContext::from_input(&input)));

    let run_list = |scripts: &[String], is_pre: bool| {
        for code in scripts {
            if code.trim().is_empty() {
                continue;
            }
            let result = run_with_timeout(code, &engine, &ctx, is_pre, timeout_ms);
            if let Err(e) = result {
                if let Ok(mut g) = ctx.lock() {
                    g.error(e);
                }
            }
        }
    };

    if phase == "pre" || phase == "both" {
        run_list(&input.pre_scripts, true);
    }
    if phase == "post" || phase == "both" {
        run_list(&input.post_scripts, false);
    }

    match Arc::try_unwrap(ctx) {
        Ok(m) => m.into_inner().unwrap().into_result(),
        Err(a) => a.lock().unwrap().clone().into_result(),
    }
}

/// Enforce wall-clock `timeout_ms` around a single script invocation.
fn run_with_timeout(
    code: &str,
    engine: &str,
    ctx: &Arc<Mutex<ScriptContext>>,
    is_pre: bool,
    timeout_ms: u64,
) -> Result<(), String> {
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    let code = code.to_string();
    let engine = engine.to_string();
    let ctx_t = ctx.clone();

    std::thread::spawn(move || {
        let result = match engine.as_str() {
            "javascript" | "js" => js_engine::run(&code, &ctx_t, is_pre),
            _ => rhai_engine::run(&code, &ctx_t, is_pre),
        };
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "Script timed out after {timeout_ms}ms (permissions.timeout_ms)"
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("Script runner terminated unexpectedly".into())
        }
    }
}

// ScriptContext needs Clone for try_unwrap fallback
impl Clone for ScriptContext {
    fn clone(&self) -> Self {
        Self {
            request: self.request.clone(),
            response: self.response.clone(),
            variables: self.variables.clone(),
            logs: self.logs.clone(),
            errors: self.errors.clone(),
            assertions: self.assertions.clone(),
            permissions: self.permissions.clone(),
            fs_root: self.fs_root.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{KeyValue, RequestBody};
    use tempfile::tempdir;

    fn base_input(engine: &str, pre: &str, post: &str, phase: &str) -> ExecuteScriptsInput {
        let dir = tempdir().unwrap();
        ExecuteScriptsInput {
            engine: engine.into(),
            pre_scripts: vec![pre.into()],
            post_scripts: vec![post.into()],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: Some(HttpResponse {
                status: 200,
                status_text: "OK".into(),
                headers: vec![KeyValue::new("X", "1")],
                body: r#"{"ok":true,"user":{"id":"42"}}"#.into(),
                body_size: 30,
                duration_ms: 12,
                content_type: Some("application/json".into()),
            }),
            variables: HashMap::new(),
            permissions: ScriptPermissions {
                allow_fs: true,
                allow_network: false,
                timeout_ms: 5_000,
            },
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: phase.into(),
        }
    }

    #[test]
    fn rhai_pre_mutates_and_post_asserts() {
        let mut input = base_input(
            "rhai",
            r#"
                req.url = "https://mutated.example/path";
                req.method = "POST";
                env.set("token", "abc");
                print("pre-ok");
            "#,
            r#"
                assert_status(200);
                assert_body_field("user.id", "42");
                assert_duration_lt(1000);
                print("post-ok");
            "#,
            "both",
        );
        // keep tempdir alive via fs_root path - recreate properly
        let dir = tempdir().unwrap();
        input.fs_root = dir.path().to_string_lossy().to_string();
        let result = execute_scripts(input);
        assert!(
            result.errors.is_empty(),
            "errors: {:?}",
            result.errors
        );
        assert!(result.logs.iter().any(|l| l.contains("pre-ok")));
        assert!(result.logs.iter().any(|l| l.contains("post-ok")));
        let req = result.request.unwrap();
        assert_eq!(req.url, "https://mutated.example/path");
        assert_eq!(req.method, "POST");
        assert_eq!(result.variables.get("token").map(String::as_str), Some("abc"));
        assert!(result.assertions.iter().all(|a| a.passed), "{:?}", result.assertions);
    }

    #[test]
    fn js_pre_mutates_and_post_asserts() {
        let dir = tempdir().unwrap();
        let input = ExecuteScriptsInput {
            engine: "javascript".into(),
            pre_scripts: vec![r#"
                req.url = "https://js.example/x";
                req.method = "PUT";
                env.set("fromJs", "1");
                console.log("js-pre");
            "#
            .into()],
            post_scripts: vec![r#"
                assert_status(200);
                assert_body_field("ok", "true");
                console.log("js-post");
            "#
            .into()],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: Some(HttpResponse {
                status: 200,
                status_text: "OK".into(),
                headers: vec![],
                body: r#"{"ok":true}"#.into(),
                body_size: 11,
                duration_ms: 5,
                content_type: None,
            }),
            variables: HashMap::new(),
            permissions: ScriptPermissions::default(),
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: "both".into(),
        };
        let result = execute_scripts(input);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(result.logs.iter().any(|l| l.contains("js-pre")));
        assert!(result.logs.iter().any(|l| l.contains("js-post")));
        assert_eq!(result.request.unwrap().method, "PUT");
        assert_eq!(result.variables.get("fromJs").map(String::as_str), Some("1"));
        assert!(result.assertions.iter().all(|a| a.passed));
    }

    #[test]
    fn tools_work() {
        assert_eq!(tools_md5("hi").len(), 32);
        assert_eq!(tools_sha256("hi").len(), 64);
        let b = tools_base64_encode("hi");
        assert_eq!(tools_base64_decode(&b).unwrap(), "hi");
        assert!(!tools_uuid().is_empty());
        let parsed = tools_json_parse(r#"{"a":1}"#).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(tools_json_stringify(&parsed), r#"{"a":1}"#);
    }

    #[test]
    fn rhai_json_tools_via_engine() {
        let dir = tempdir().unwrap();
        let input = ExecuteScriptsInput {
            engine: "rhai".into(),
            pre_scripts: vec![
                r#"
                let j = tools.json_parse("{\"n\":42}");
                print(j);
                let s = tools.json_stringify("{\"n\":42}");
                print(s);
                "#
                .into(),
            ],
            post_scripts: vec![],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: None,
            variables: HashMap::new(),
            permissions: ScriptPermissions::default(),
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: "pre".into(),
        };
        let result = execute_scripts(input);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(
            result.logs.iter().any(|l| l.contains("42")),
            "logs: {:?}",
            result.logs
        );
    }

    #[test]
    fn js_pre_mutates_headers_and_query() {
        let dir = tempdir().unwrap();
        let input = ExecuteScriptsInput {
            engine: "javascript".into(),
            pre_scripts: vec![r#"
                req.headers = [{ key: "X-Foo", value: "bar", enabled: true }];
                req.query = [{ key: "q", value: "1", enabled: true }];
                req.url = "https://js.example/headers";
            "#
            .into()],
            post_scripts: vec![],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: None,
            variables: HashMap::new(),
            permissions: ScriptPermissions::default(),
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: "pre".into(),
        };
        let result = execute_scripts(input);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let req = result.request.expect("request present");
        assert_eq!(req.url, "https://js.example/headers");
        assert!(
            req.headers.iter().any(|h| h.key == "X-Foo" && h.value == "bar"),
            "headers: {:?}",
            req.headers
        );
        assert!(
            req.query.iter().any(|q| q.key == "q" && q.value == "1"),
            "query: {:?}",
            req.query
        );
    }

    #[test]
    fn script_timeout_enforced_rhai() {
        let dir = tempdir().unwrap();
        let input = ExecuteScriptsInput {
            engine: "rhai".into(),
            pre_scripts: vec!["while true { let x = 1; }".into()],
            post_scripts: vec![],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: None,
            variables: HashMap::new(),
            permissions: ScriptPermissions {
                allow_fs: false,
                allow_network: false,
                timeout_ms: 80,
            },
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: "pre".into(),
        };
        let result = execute_scripts(input);
        assert!(
            !result.errors.is_empty(),
            "expected timeout/max-ops error, got none"
        );
        let joined = result.errors.join(" | ").to_lowercase();
        assert!(
            joined.contains("timed out")
                || joined.contains("timeout")
                || joined.contains("operations")
                || joined.contains("maximum"),
            "unexpected errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn script_timeout_enforced_js() {
        let dir = tempdir().unwrap();
        let input = ExecuteScriptsInput {
            engine: "javascript".into(),
            pre_scripts: vec!["while (true) { /* spin */ }".into()],
            post_scripts: vec![],
            request: MutableRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                query: vec![],
                body: RequestBody {
                    body_type: "none".into(),
                    content: String::new(),
                },
            },
            response: None,
            variables: HashMap::new(),
            permissions: ScriptPermissions {
                allow_fs: false,
                allow_network: false,
                timeout_ms: 80,
            },
            fs_root: dir.path().to_string_lossy().to_string(),
            phase: "pre".into(),
        };
        let result = execute_scripts(input);
        assert!(!result.errors.is_empty(), "expected JS timeout error");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.to_lowercase().contains("timed out")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn fs_permissions() {
        let dir = tempdir().unwrap();
        let mut input = base_input("rhai", r#"fs.write("a.txt", "hello"); print(fs.read("a.txt"));"#, "", "pre");
        input.fs_root = dir.path().to_string_lossy().to_string();
        let result = execute_scripts(input);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(result.logs.iter().any(|l| l.contains("hello")));
    }
}
