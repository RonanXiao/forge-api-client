mod auth;
mod codegen;
mod cookies;
mod env_interp;
mod form_fields;
mod http;
mod import;
mod models;
mod scripts;
mod storage;

use models::*;
use std::collections::HashMap;

#[tauri::command]
async fn send_http_request(mut input: SendRequestInput) -> Result<HttpResponse, String> {
    // Merge active env vars if not provided
    if input.variables.is_empty() {
        if let Ok(vars) = storage::active_env_vars() {
            input.variables = vars;
        }
    }
    if input.proxy.is_none() {
        if let Ok(cfg) = storage::load_config() {
            input.proxy = Some(cfg.proxy);
        }
    }
    let mut jar = storage::load_cookies().unwrap_or_default();
    let resp = http::send_request(input, Some(&mut jar)).await?;
    let _ = storage::save_cookies(&jar);
    Ok(resp)
}

#[tauri::command]
fn list_collections() -> Result<Vec<Collection>, String> {
    storage::list_collections()
}

#[tauri::command]
fn save_collection(collection: Collection) -> Result<(), String> {
    storage::save_collection(&collection)
}

#[tauri::command]
fn save_collection_format(collection: Collection, format: String) -> Result<(), String> {
    storage::save_collection_as(&collection, &format)
}

#[tauri::command]
fn delete_collection(id: String) -> Result<(), String> {
    storage::delete_collection(&id)
}

/// Add a request to a collection (root or folder). Persists and returns full updated collection + new request id.
#[tauri::command]
fn add_request(
    collection_id: String,
    parent_id: Option<String>,
    name: Option<String>,
) -> Result<AddRequestResult, String> {
    let name = name.unwrap_or_else(|| "New Request".into());
    let (collection, request_id) =
        storage::add_request_and_save(&collection_id, parent_id.as_deref(), &name)?;
    Ok(AddRequestResult {
        collection,
        request_id,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AddRequestResult {
    collection: Collection,
    request_id: String,
}

#[tauri::command]
fn load_history() -> Result<Vec<HistoryEntry>, String> {
    storage::load_history()
}

#[tauri::command]
fn append_history(entry: HistoryEntry) -> Result<Vec<HistoryEntry>, String> {
    storage::append_history(entry)
}

#[tauri::command]
fn clear_history() -> Result<(), String> {
    storage::clear_history()
}

#[tauri::command]
fn get_workspace_path() -> Result<String, String> {
    storage::get_workspace_path()
}

#[tauri::command]
fn set_workspace_path(path: Option<String>) -> Result<String, String> {
    storage::set_workspace_path(path)
}

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    storage::load_config()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    storage::save_config(&config)
}

#[tauri::command]
fn load_environments() -> Result<EnvironmentFile, String> {
    storage::load_environments()
}

#[tauri::command]
fn save_environments(file: EnvironmentFile) -> Result<(), String> {
    storage::save_environments(&file)
}

#[tauri::command]
fn load_cookies() -> Result<Vec<CookieEntry>, String> {
    storage::load_cookies()
}

#[tauri::command]
fn save_cookies(cookies: Vec<CookieEntry>) -> Result<(), String> {
    storage::save_cookies(&cookies)
}

#[tauri::command]
fn delete_cookie(id: String) -> Result<Vec<CookieEntry>, String> {
    let mut jar = storage::load_cookies()?;
    cookies::delete_cookie(&mut jar, &id);
    storage::save_cookies(&jar)?;
    Ok(jar)
}

#[tauri::command]
fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[tauri::command]
fn interpolate_text(text: String, variables: HashMap<String, String>) -> String {
    env_interp::interpolate(&text, &variables)
}

#[tauri::command]
fn execute_scripts(input: ExecuteScriptsInput) -> ScriptRunResult {
    scripts::execute_scripts(input)
}

#[tauri::command]
fn import_curl(text: String) -> Result<HttpRequest, String> {
    import::parse_curl(&text)
}

#[tauri::command]
fn import_postman(json: String) -> Result<Collection, String> {
    import::import_postman_v21(&json)
}

#[tauri::command]
fn generate_code(input: CodegenInput, language: String) -> Result<String, String> {
    match language.to_lowercase().as_str() {
        "curl" => Ok(codegen::generate_curl(&input)),
        "javascript" | "fetch" | "js" => Ok(codegen::generate_fetch(&input)),
        "python" | "py" => Ok(codegen::generate_python(&input)),
        other => Err(format!("Unknown language: {other}")),
    }
}

#[tauri::command]
fn search_requests(query: String) -> Result<Vec<SearchHit>, String> {
    let cols = storage::list_collections()?;
    Ok(storage::search_collections(&cols, &query))
}

#[tauri::command]
fn tree_rename(mut collection: Collection, item_id: String, name: String) -> Result<Collection, String> {
    if !storage::rename_item(&mut collection.items, &item_id, &name) {
        return Err("Item not found".into());
    }
    storage::save_collection(&collection)?;
    Ok(collection)
}

#[tauri::command]
fn tree_delete(mut collection: Collection, item_id: String) -> Result<Collection, String> {
    if !storage::delete_item(&mut collection.items, &item_id) {
        return Err("Item not found".into());
    }
    storage::save_collection(&collection)?;
    Ok(collection)
}

#[tauri::command]
fn tree_reorder(
    mut collection: Collection,
    parent_id: Option<String>,
    item_id: String,
    to_index: usize,
) -> Result<Collection, String> {
    if !storage::reorder_item(
        &mut collection.items,
        parent_id.as_deref(),
        &item_id,
        to_index,
    ) {
        return Err("Reorder failed".into());
    }
    storage::save_collection(&collection)?;
    Ok(collection)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            send_http_request,
            list_collections,
            save_collection,
            save_collection_format,
            delete_collection,
            add_request,
            load_history,
            append_history,
            clear_history,
            get_workspace_path,
            set_workspace_path,
            get_config,
            save_config,
            load_environments,
            save_environments,
            load_cookies,
            save_cookies,
            delete_cookie,
            new_id,
            interpolate_text,
            execute_scripts,
            import_curl,
            import_postman,
            generate_code,
            search_requests,
            tree_rename,
            tree_delete,
            tree_reorder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
