import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  Collection,
  CookieEntry,
  EnvironmentFile,
  HistoryEntry,
  HttpRequest,
  HttpResponse,
  ScriptRunResult,
  SearchHit,
  SendRequestInput,
} from "./types";

export type { ScriptRunResult };

export interface ExecuteScriptsInput {
  engine: string;
  preScripts: string[];
  postScripts: string[];
  request: {
    method: string;
    url: string;
    headers: { key: string; value: string; enabled: boolean }[];
    query: { key: string; value: string; enabled: boolean }[];
    body: { type: string; content: string };
  };
  response?: HttpResponse | null;
  variables: Record<string, string>;
  permissions: {
    allowFs: boolean;
    allowNetwork: boolean;
    timeoutMs: number;
  };
  fsRoot: string;
  phase: string;
}

export interface CodegenInput {
  method: string;
  url: string;
  headers: { key: string; value: string; enabled: boolean }[];
  query: { key: string; value: string; enabled: boolean }[];
  body: {
    type: string;
    content: string;
    language?: string | null;
  };
  auth: {
    type: string;
    token: string;
    username: string;
    password: string;
    key: string;
    value: string;
    addTo: string;
  };
}

export async function sendHttpRequest(
  input: SendRequestInput,
): Promise<HttpResponse> {
  return invoke("send_http_request", { input });
}

export async function listCollections(): Promise<Collection[]> {
  return invoke("list_collections");
}

export async function saveCollection(collection: Collection): Promise<void> {
  return invoke("save_collection", { collection });
}

export async function deleteCollection(id: string): Promise<void> {
  return invoke("delete_collection", { id });
}

export interface AddRequestResult {
  collection: Collection;
  requestId: string;
}

/** Backend-persisted add request — single source of truth for the + button. */
export async function addRequest(
  collectionId: string,
  parentId?: string | null,
  name?: string,
): Promise<AddRequestResult> {
  return invoke("add_request", {
    collectionId,
    parentId: parentId ?? null,
    name: name ?? "New Request",
  });
}

export async function loadHistory(): Promise<HistoryEntry[]> {
  return invoke("load_history");
}

export async function appendHistory(
  entry: HistoryEntry,
): Promise<HistoryEntry[]> {
  return invoke("append_history", { entry });
}

export async function clearHistory(): Promise<void> {
  return invoke("clear_history");
}

export async function getWorkspacePath(): Promise<string> {
  return invoke("get_workspace_path");
}

export async function setWorkspacePath(path: string | null): Promise<string> {
  return invoke("set_workspace_path", { path });
}

export async function newId(): Promise<string> {
  return invoke("new_id");
}

export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config", { config });
}

export async function loadEnvironments(): Promise<EnvironmentFile> {
  return invoke("load_environments");
}

export async function saveEnvironments(file: EnvironmentFile): Promise<void> {
  return invoke("save_environments", { file });
}

export async function loadCookies(): Promise<CookieEntry[]> {
  return invoke("load_cookies");
}

export async function saveCookies(cookies: CookieEntry[]): Promise<void> {
  return invoke("save_cookies", { cookies });
}

export async function deleteCookie(id: string): Promise<CookieEntry[]> {
  return invoke("delete_cookie", { id });
}

export async function executeScripts(
  input: ExecuteScriptsInput,
): Promise<ScriptRunResult> {
  return invoke("execute_scripts", { input });
}

export async function importCurl(text: string): Promise<HttpRequest> {
  return invoke("import_curl", { text });
}

export async function importPostman(json: string): Promise<Collection> {
  return invoke("import_postman", { json });
}

export async function generateCode(
  input: CodegenInput,
  language: string,
): Promise<string> {
  return invoke("generate_code", { input, language });
}

export async function searchRequests(query: string): Promise<SearchHit[]> {
  return invoke("search_requests", { query });
}

export async function treeRename(
  collection: Collection,
  itemId: string,
  name: string,
): Promise<Collection> {
  return invoke("tree_rename", { collection, itemId, name });
}

export async function treeDelete(
  collection: Collection,
  itemId: string,
): Promise<Collection> {
  return invoke("tree_delete", { collection, itemId });
}

export async function treeReorder(
  collection: Collection,
  parentId: string | null,
  itemId: string,
  toIndex: number,
): Promise<Collection> {
  return invoke("tree_reorder", { collection, parentId, itemId, toIndex });
}

export async function interpolateText(
  text: string,
  variables: Record<string, string>,
): Promise<string> {
  return invoke("interpolate_text", { text, variables });
}
