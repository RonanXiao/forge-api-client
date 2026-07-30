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
    #[serde(rename = "type")]
    pub body_type: String, // none | json | form | raw | multipart | binary
    #[serde(default)]
    pub content: String,
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
    "dark".into()
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
