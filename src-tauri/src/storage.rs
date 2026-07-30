use crate::models::{
    AppConfig, Collection, CookieEntry, Environment, EnvironmentFile, HistoryEntry,
};
use std::fs;
use std::path::{Path, PathBuf};

const COLLECTIONS_DIR: &str = "collections";
const HISTORY_FILE: &str = "history.json";
const CONFIG_FILE: &str = "config.json";
const ENV_FILE: &str = "environments.json";
const COOKIES_FILE: &str = "cookies.json";
const MAX_HISTORY: usize = 100;

/// Override app data root for tests.
static TEST_ROOT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

#[cfg(test)]
pub fn set_test_root(path: Option<PathBuf>) {
    *TEST_ROOT.lock().unwrap() = path;
}

fn app_data_dir() -> Result<PathBuf, String> {
    if let Some(ref p) = *TEST_ROOT.lock().unwrap() {
        fs::create_dir_all(p).map_err(|e| format!("Failed to create test data dir: {e}"))?;
        return Ok(p.clone());
    }
    let base = dirs::data_dir().ok_or("Could not resolve data directory")?;
    let dir = base.join("Forge");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
    Ok(dir)
}

fn workspace_dir(config: &AppConfig) -> Result<PathBuf, String> {
    if let Some(ref path) = config.workspace_path {
        let p = PathBuf::from(path);
        fs::create_dir_all(&p).map_err(|e| format!("Failed to create workspace: {e}"))?;
        Ok(p)
    } else {
        app_data_dir()
    }
}

fn collections_path(workspace: &Path) -> PathBuf {
    let p = workspace.join(COLLECTIONS_DIR);
    let _ = fs::create_dir_all(&p);
    p
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = app_data_dir()?.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Read config failed: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Parse config failed: {e}"))
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = app_data_dir()?.join(CONFIG_FILE);
    let raw =
        serde_json::to_string_pretty(config).map_err(|e| format!("Serialize config failed: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Write config failed: {e}"))
}

pub fn list_collections() -> Result<Vec<Collection>, String> {
    let config = load_config()?;
    let dir = collections_path(&workspace_dir(&config)?);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut collections = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("Read collections dir failed: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "json" && ext != "yaml" && ext != "yml" {
            continue;
        }
        match load_collection_file(&path) {
            Ok(c) => collections.push(c),
            Err(e) => eprintln!("Skip collection {:?}: {e}", path),
        }
    }
    collections.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(collections)
}

pub fn load_collection_file(path: &Path) -> Result<Collection, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("Read collection failed: {e}"))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("json");
    if ext == "yaml" || ext == "yml" {
        serde_yaml::from_str(&raw).map_err(|e| format!("Parse YAML collection failed: {e}"))
    } else {
        serde_json::from_str(&raw).map_err(|e| format!("Parse collection failed: {e}"))
    }
}

pub fn save_collection(collection: &Collection) -> Result<(), String> {
    save_collection_as(collection, "json")
}

pub fn save_collection_as(collection: &Collection, format: &str) -> Result<(), String> {
    let config = load_config()?;
    let dir = collections_path(&workspace_dir(&config)?);
    let ext = if format == "yaml" || format == "yml" {
        "yaml"
    } else {
        "json"
    };
    let path = dir.join(format!("{}.{}", sanitize_filename(&collection.id), ext));
    let raw = if ext == "yaml" {
        serde_yaml::to_string(collection).map_err(|e| format!("Serialize YAML failed: {e}"))?
    } else {
        serde_json::to_string_pretty(collection)
            .map_err(|e| format!("Serialize collection failed: {e}"))?
    };
    fs::write(&path, raw).map_err(|e| format!("Write collection failed: {e}"))
}

pub fn delete_collection(id: &str) -> Result<(), String> {
    let config = load_config()?;
    let dir = collections_path(&workspace_dir(&config)?);
    for ext in ["json", "yaml", "yml"] {
        let path = dir.join(format!("{}.{}", sanitize_filename(id), ext));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Delete collection failed: {e}"))?;
        }
    }
    Ok(())
}

pub fn load_history() -> Result<Vec<HistoryEntry>, String> {
    let path = app_data_dir()?.join(HISTORY_FILE);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Read history failed: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Parse history failed: {e}"))
}

pub fn append_history(entry: HistoryEntry) -> Result<Vec<HistoryEntry>, String> {
    let mut history = load_history().unwrap_or_default();
    history.insert(0, entry);
    history.truncate(MAX_HISTORY);
    let path = app_data_dir()?.join(HISTORY_FILE);
    let raw = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Serialize history failed: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Write history failed: {e}"))?;
    Ok(history)
}

pub fn clear_history() -> Result<(), String> {
    let path = app_data_dir()?.join(HISTORY_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Clear history failed: {e}"))?;
    }
    Ok(())
}

pub fn load_environments() -> Result<EnvironmentFile, String> {
    let path = app_data_dir()?.join(ENV_FILE);
    if !path.exists() {
        return Ok(EnvironmentFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Read environments failed: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Parse environments failed: {e}"))
}

pub fn save_environments(file: &EnvironmentFile) -> Result<(), String> {
    let path = app_data_dir()?.join(ENV_FILE);
    let raw = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Serialize environments failed: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Write environments failed: {e}"))
}

pub fn load_cookies() -> Result<Vec<CookieEntry>, String> {
    let path = app_data_dir()?.join(COOKIES_FILE);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Read cookies failed: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Parse cookies failed: {e}"))
}

pub fn save_cookies(cookies: &[CookieEntry]) -> Result<(), String> {
    let path = app_data_dir()?.join(COOKIES_FILE);
    let raw =
        serde_json::to_string_pretty(cookies).map_err(|e| format!("Serialize cookies failed: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Write cookies failed: {e}"))
}

pub fn get_workspace_path() -> Result<String, String> {
    let config = load_config()?;
    Ok(workspace_dir(&config)?.to_string_lossy().to_string())
}

pub fn set_workspace_path(path: Option<String>) -> Result<String, String> {
    let mut config = load_config()?;
    config.workspace_path = path;
    save_config(&config)?;
    get_workspace_path()
}

pub fn active_env_vars() -> Result<std::collections::HashMap<String, String>, String> {
    let file = load_environments()?;
    let config = load_config()?;
    let id = config
        .active_env_id
        .clone()
        .or(file.active_id.clone());
    let Some(id) = id else {
        return Ok(Default::default());
    };
    let env = file
        .environments
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .unwrap_or(Environment {
            id,
            name: String::new(),
            variables: vec![],
        });
    Ok(crate::env_interp::vars_from_kv(&env.variables))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Create a minimal new request item (root of a collection or folder).
pub fn make_request_item(id: &str, name: &str) -> CollectionItemMut {
    CollectionItemMut {
        id: id.to_string(),
        item_type: "request".into(),
        name: name.to_string(),
        children: None,
        request: Some(crate::models::HttpRequest {
            id: id.to_string(),
            name: name.to_string(),
            method: "GET".into(),
            url: "https://httpbin.org/get".into(),
            headers: vec![],
            query: vec![],
            body: crate::models::RequestBody {
                body_type: "none".into(),
                content: String::new(),
                language: None,
            },
            auth: crate::models::AuthConfig {
                auth_type: "none".into(),
                ..Default::default()
            },
            config: crate::models::RequestConfig {
                timeout_ms: Some(30_000),
                max_redirects: Some(10),
                follow_redirects: Some(true),
            },
            scripts: Default::default(),
        }),
        scripts: None,
    }
}

/// Append a new request under collection root or under a folder (`parent_id`).
/// Returns the updated collection and the new item id.
pub fn add_request_to_collection(
    collection: &mut Collection,
    parent_id: Option<&str>,
    request_id: &str,
    name: &str,
) -> Result<String, String> {
    let item = make_request_item(request_id, name);
    if let Some(pid) = parent_id {
        if !append_to_folder(&mut collection.items, pid, item) {
            return Err(format!("Folder not found: {pid}"));
        }
    } else {
        collection.items.push(item);
    }
    Ok(request_id.to_string())
}

fn append_to_folder(items: &mut [CollectionItemMut], folder_id: &str, item: CollectionItemMut) -> bool {
    for it in items.iter_mut() {
        if it.id == folder_id && it.item_type == "folder" {
            let children = it.children.get_or_insert_with(Vec::new);
            children.push(item);
            return true;
        }
        if let Some(ref mut children) = it.children {
            if append_to_folder(children, folder_id, item.clone()) {
                return true;
            }
        }
    }
    false
}

/// Load collection by id, add request, save, return updated collection.
pub fn add_request_and_save(
    collection_id: &str,
    parent_id: Option<&str>,
    name: &str,
) -> Result<(Collection, String), String> {
    let mut collections = list_collections()?;
    let col = collections
        .iter_mut()
        .find(|c| c.id == collection_id)
        .ok_or_else(|| format!("Collection not found: {collection_id}"))?;
    let req_id = uuid::Uuid::new_v4().to_string();
    add_request_to_collection(col, parent_id, &req_id, name)?;
    let saved = col.clone();
    save_collection(&saved)?;
    Ok((saved, req_id))
}

/// Tree helpers
pub fn rename_item(items: &mut [CollectionItemMut], id: &str, name: &str) -> bool {
    for item in items.iter_mut() {
        if item.id == id {
            item.name = name.to_string();
            // Keep embedded request name in sync with tree label
            if let Some(ref mut req) = item.request {
                req.name = name.to_string();
            }
            return true;
        }
        if let Some(ref mut children) = item.children {
            if rename_item(children, id, name) {
                return true;
            }
        }
    }
    false
}

// Re-export CollectionItem for tree ops via type alias
use crate::models::CollectionItem as CollectionItemMut;

pub fn delete_item(items: &mut Vec<CollectionItemMut>, id: &str) -> bool {
    if let Some(pos) = items.iter().position(|i| i.id == id) {
        items.remove(pos);
        return true;
    }
    for item in items.iter_mut() {
        if let Some(ref mut children) = item.children {
            if delete_item(children, id) {
                return true;
            }
        }
    }
    false
}

/// Reorder: move item `id` to index `to_index` within the same parent list identified by parent_id (None = root).
pub fn reorder_item(
    items: &mut Vec<CollectionItemMut>,
    parent_id: Option<&str>,
    id: &str,
    to_index: usize,
) -> bool {
    let list = if let Some(pid) = parent_id {
        find_children_mut(items, pid)
    } else {
        Some(items)
    };
    let Some(list) = list else {
        return false;
    };
    let Some(from) = list.iter().position(|i| i.id == id) else {
        return false;
    };
    let item = list.remove(from);
    let idx = to_index.min(list.len());
    list.insert(idx, item);
    true
}

fn find_children_mut<'a>(
    items: &'a mut [CollectionItemMut],
    parent_id: &str,
) -> Option<&'a mut Vec<CollectionItemMut>> {
    for item in items.iter_mut() {
        if item.id == parent_id {
            if item.children.is_none() {
                item.children = Some(vec![]);
            }
            return item.children.as_mut();
        }
        if let Some(ref mut children) = item.children {
            if let Some(found) = find_children_mut(children, parent_id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn search_collections(collections: &[Collection], query: &str) -> Vec<crate::models::SearchHit> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let mut hits = vec![];
    for col in collections {
        walk_search(&col.id, &col.name, &col.items, "", &q, &mut hits);
    }
    hits
}

fn walk_search(
    col_id: &str,
    col_name: &str,
    items: &[CollectionItemMut],
    path: &str,
    q: &str,
    hits: &mut Vec<crate::models::SearchHit>,
) {
    for item in items {
        let p = if path.is_empty() {
            item.name.clone()
        } else {
            format!("{path} / {}", item.name)
        };
        if item.item_type == "request" {
            if let Some(ref req) = item.request {
                if item.name.to_lowercase().contains(q) || req.url.to_lowercase().contains(q) {
                    hits.push(crate::models::SearchHit {
                        collection_id: col_id.to_string(),
                        collection_name: col_name.to_string(),
                        item_id: item.id.clone(),
                        name: item.name.clone(),
                        method: req.method.clone(),
                        url: req.url.clone(),
                        path: p.clone(),
                    });
                }
            }
        }
        if let Some(ref children) = item.children {
            walk_search(col_id, col_name, children, &p, q, hits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CollectionItem, HttpRequest, KeyValue};
    use std::sync::Mutex;
    use tempfile::tempdir;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn collection_json_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        set_test_root(Some(dir.path().to_path_buf()));
        let col = Collection {
            id: "col1".into(),
            name: "Test".into(),
            version: "1.0".into(),
            items: vec![CollectionItem {
                id: "r1".into(),
                item_type: "request".into(),
                name: "R".into(),
                children: None,
                request: Some(HttpRequest {
                    id: "r1".into(),
                    name: "R".into(),
                    method: "GET".into(),
                    url: "https://example.com".into(),
                    ..Default::default()
                }),
                scripts: None,
            }],
            scripts: None,
            engine: None,
        };
        save_collection(&col).unwrap();
        let loaded = list_collections().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test");
        assert_eq!(loaded[0].items[0].request.as_ref().unwrap().url, "https://example.com");

        // yaml too
        save_collection_as(&col, "yaml").unwrap();
        let yaml_path = dir
            .path()
            .join("collections")
            .join("col1.yaml");
        assert!(yaml_path.exists());
        set_test_root(None);
    }

    #[test]
    fn env_and_cookies_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        set_test_root(Some(dir.path().to_path_buf()));
        let envs = EnvironmentFile {
            environments: vec![Environment {
                id: "e1".into(),
                name: "dev".into(),
                variables: vec![KeyValue::new("host", "localhost")],
            }],
            active_id: Some("e1".into()),
        };
        save_environments(&envs).unwrap();
        let loaded = load_environments().unwrap();
        assert_eq!(loaded.environments[0].name, "dev");

        let cookies = vec![CookieEntry {
            id: "c1".into(),
            name: "a".into(),
            value: "b".into(),
            domain: "localhost".into(),
            path: "/".into(),
            secure: false,
            http_only: false,
            expires: None,
        }];
        save_cookies(&cookies).unwrap();
        assert_eq!(load_cookies().unwrap()[0].value, "b");
        set_test_root(None);
    }

    #[test]
    fn tree_rename_delete_reorder() {
        let mut items = vec![
            CollectionItem {
                id: "a".into(),
                item_type: "request".into(),
                name: "A".into(),
                children: None,
                request: None,
                scripts: None,
            },
            CollectionItem {
                id: "b".into(),
                item_type: "folder".into(),
                name: "B".into(),
                children: Some(vec![CollectionItem {
                    id: "c".into(),
                    item_type: "request".into(),
                    name: "C".into(),
                    children: None,
                    request: None,
                    scripts: None,
                }]),
                request: None,
                scripts: None,
            },
        ];
        assert!(rename_item(&mut items, "c", "C2"));
        assert_eq!(items[1].children.as_ref().unwrap()[0].name, "C2");
        assert!(reorder_item(&mut items, None, "b", 0));
        assert_eq!(items[0].id, "b");
        assert!(delete_item(&mut items, "c"));
        assert!(items[0].children.as_ref().unwrap().is_empty());
    }

    #[test]
    fn add_request_to_collection_root_and_folder_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        set_test_root(Some(dir.path().to_path_buf()));

        let mut col = Collection {
            id: "col-add".into(),
            name: "AddTest".into(),
            version: "1.0".into(),
            items: vec![CollectionItem {
                id: "folder1".into(),
                item_type: "folder".into(),
                name: "Folder".into(),
                children: Some(vec![]),
                request: None,
                scripts: None,
            }],
            scripts: None,
            engine: None,
        };
        save_collection(&col).unwrap();

        // root request
        let id1 = uuid::Uuid::new_v4().to_string();
        add_request_to_collection(&mut col, None, &id1, "Root Req").unwrap();
        assert_eq!(col.items.len(), 2);
        assert_eq!(col.items[1].item_type, "request");
        assert!(col.items[1].request.is_some());

        // folder request
        let id2 = uuid::Uuid::new_v4().to_string();
        add_request_to_collection(&mut col, Some("folder1"), &id2, "Nested Req").unwrap();
        let folder = col.items.iter().find(|i| i.id == "folder1").unwrap();
        assert_eq!(folder.children.as_ref().unwrap().len(), 1);
        assert_eq!(folder.children.as_ref().unwrap()[0].name, "Nested Req");

        save_collection(&col).unwrap();
        let loaded = list_collections().unwrap();
        let again = loaded.iter().find(|c| c.id == "col-add").unwrap();
        assert_eq!(again.items.len(), 2);
        let f = again.items.iter().find(|i| i.id == "folder1").unwrap();
        assert_eq!(f.children.as_ref().unwrap()[0].id, id2);

        // full save path
        let (updated, rid) =
            add_request_and_save("col-add", None, "ViaSave").unwrap();
        assert_eq!(updated.items.len(), 3);
        assert!(!rid.is_empty());
        let reloaded = list_collections().unwrap();
        let c = reloaded.iter().find(|c| c.id == "col-add").unwrap();
        assert!(c.items.iter().any(|i| i.name == "ViaSave"));

        set_test_root(None);
    }
}
