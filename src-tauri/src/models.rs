use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestBody {
    /// Postman modes: none | form-data | urlencoded | raw | binary
    /// Legacy aliases also accepted: json | form | multipart | formdata
    #[serde(rename = "type")]
    pub body_type: String,
    #[serde(default)]
    pub content: String,
    /// Raw language subtype (Postman): text | javascript | json | html | xml
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl RequestBody {
    pub fn none() -> Self {
        Self {
            body_type: "none".into(),
            content: String::new(),
            language: None,
        }
    }

    pub fn with(body_type: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            body_type: body_type.into(),
            content: content.into(),
            language: None,
        }
    }

    pub fn with_language(
        body_type: impl Into<String>,
        content: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            body_type: body_type.into(),
            content: content.into(),
            language: Some(language.into()),
        }
    }

    /// Canonical Postman body mode for HTTP send / codegen.
    /// Returns: none | form-data | urlencoded | raw | binary
    /// Special: "json" when type is legacy json or raw+language=json (for Content-Type).
    pub fn send_mode(&self) -> &'static str {
        let t = self.body_type.to_lowercase();
        match t.as_str() {
            "form-data" | "formdata" | "multipart" => "form-data",
            "urlencoded" | "x-www-form-urlencoded" | "form" => "urlencoded",
            "json" => "json",
            "raw" => {
                if self
                    .language
                    .as_deref()
                    .map(|l| l.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
                {
                    "json"
                } else {
                    "raw"
                }
            }
            "binary" | "file" => "binary",
            "none" | "" => "none",
            _ => "raw",
        }
    }

    /// Normalize legacy type names to Postman storage format.
    pub fn normalize(mut self) -> Self {
        let t = self.body_type.to_lowercase();
        match t.as_str() {
            "multipart" | "formdata" => {
                self.body_type = "form-data".into();
                self.language = None;
            }
            "form" | "x-www-form-urlencoded" => {
                self.body_type = "urlencoded".into();
                self.language = None;
            }
            "json" => {
                self.body_type = "raw".into();
                if self.language.is_none() {
                    self.language = Some("json".into());
                }
            }
            "file" => {
                self.body_type = "binary".into();
                self.language = None;
            }
            "raw" => {
                if self.language.is_none() {
                    self.language = Some("text".into());
                }
            }
            "form-data" | "urlencoded" | "binary" | "none" => {
                if self.body_type != t {
                    self.body_type = t;
                }
                if self.body_type != "raw" {
                    self.language = None;
                }
            }
            _ => {}
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    /// none | bearer | basic | apikey
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
    /// header | query
    #[serde(default = "default_header")]
    pub add_to: String,
}

fn default_header() -> String {
    "header".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestConfig {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_redirects: Option<u32>,
    #[serde(default)]
    pub follow_redirects: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptBlock {
    #[serde(default)]
    pub pre_request: String,
    #[serde(default)]
    pub post_response: String,
    /// rhai | javascript | inherit
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub config: RequestConfig,
    #[serde(default)]
    pub scripts: ScriptBlock,
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "New Request".into(),
            method: "GET".into(),
            url: String::new(),
            headers: vec![],
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
            config: RequestConfig::default(),
            scripts: ScriptBlock::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String, // folder | request
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<CollectionItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<HttpRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scripts: Option<ScriptBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scripts: Option<ScriptBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    /// system | none | manual
    #[serde(default = "default_proxy_mode")]
    pub mode: String,
    #[serde(default)]
    pub http: Option<String>,
    #[serde(default)]
    pub https: Option<String>,
    #[serde(default)]
    pub socks: Option<String>,
}

fn default_proxy_mode() -> String {
    "system".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptPermissions {
    #[serde(default = "default_true")]
    pub allow_fs: bool,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default = "default_script_timeout")]
    pub timeout_ms: u64,
}

fn default_script_timeout() -> u64 {
    5_000
}

impl Default for ScriptPermissions {
    fn default() -> Self {
        Self {
            allow_fs: true,
            allow_network: false,
            timeout_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub workspace_path: Option<String>,
    /// rhai | javascript
    #[serde(default = "default_engine")]
    pub default_engine: String,
    #[serde(default)]
    pub active_env_id: Option<String>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub script_permissions: ScriptPermissions,
    /// dark | light
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_engine() -> String {
    "rhai".into()
}

fn default_theme() -> String {
    "light".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspace_path: None,
            default_engine: default_engine(),
            active_env_id: None,
            proxy: ProxyConfig::default(),
            script_permissions: ScriptPermissions::default(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<KeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentFile {
    #[serde(default)]
    pub environments: Vec<Environment>,
    #[serde(default)]
    pub active_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CookieEntry {
    pub id: String,
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub expires: Option<String>,
}

fn default_path() -> String {
    "/".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub config: RequestConfig,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Environment variable map already resolved by caller, or empty
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// When true, skip cookie jar (tests)
    #[serde(default)]
    pub skip_cookies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub body_size: u64,
    pub duration_ms: u64,
    pub content_type: Option<String>,
    /// curl -v style debug trace (connect, request, response headers, timing)
    #[serde(default)]
    pub verbose: String,
    /// Short network/send error (timeout, DNS, etc.). Full trace stays in `verbose`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub timestamp: String,
    pub request: SendRequestInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRunResult {
    pub logs: Vec<String>,
    pub errors: Vec<String>,
    pub assertions: Vec<AssertionResult>,
    pub variables: HashMap<String, String>,
    pub request: Option<MutableRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MutableRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub query: Vec<KeyValue>,
    pub body: RequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteScriptsInput {
    pub engine: String,
    pub pre_scripts: Vec<String>,
    pub post_scripts: Vec<String>,
    pub request: MutableRequest,
    pub response: Option<HttpResponse>,
    pub variables: HashMap<String, String>,
    pub permissions: ScriptPermissions,
    /// Root for fs operations (workspace)
    pub fs_root: String,
    pub phase: String, // pre | post | both
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodegenInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub collection_id: String,
    pub collection_name: String,
    pub item_id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub path: String,
}
