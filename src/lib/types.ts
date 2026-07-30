export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

/** Postman body modes (legacy aliases: json, form, multipart are normalized on load). */
export type BodyType =
  | "none"
  | "form-data"
  | "urlencoded"
  | "raw"
  | "binary";

/** Language subtype when body type is raw (Postman-style). */
export type RawLanguage = "text" | "javascript" | "json" | "html" | "xml";

export interface RequestBody {
  type: BodyType;
  content: string;
  /** Only used when type === "raw". */
  language?: RawLanguage | null;
}

export type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE"
  | "HEAD"
  | "OPTIONS";

export interface AuthConfig {
  type: "none" | "bearer" | "basic" | "apikey";
  token: string;
  username: string;
  password: string;
  key: string;
  value: string;
  addTo: "header" | "query";
}

export interface RequestConfig {
  timeoutMs?: number | null;
  maxRedirects?: number | null;
  followRedirects?: boolean | null;
}

export interface ScriptBlock {
  preRequest: string;
  postResponse: string;
  engine?: string | null;
}

export interface HttpRequest {
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  headers: KeyValue[];
  query: KeyValue[];
  body: RequestBody;
  auth: AuthConfig;
  config: RequestConfig;
  scripts: ScriptBlock;
}

export interface CollectionItem {
  id: string;
  type: "folder" | "request";
  name: string;
  children?: CollectionItem[];
  request?: HttpRequest;
  scripts?: ScriptBlock | null;
}

export interface Collection {
  id: string;
  name: string;
  version: string;
  items: CollectionItem[];
  scripts?: ScriptBlock | null;
  engine?: string | null;
}

export interface SendRequestInput {
  method: string;
  url: string;
  headers: KeyValue[];
  query: KeyValue[];
  body: RequestBody;
  auth?: AuthConfig;
  config?: RequestConfig;
  timeoutMs?: number;
  variables?: Record<string, string>;
  proxy?: ProxyConfig | null;
  skipCookies?: boolean;
}

export interface HttpResponse {
  status: number;
  statusText: string;
  headers: KeyValue[];
  body: string;
  bodySize: number;
  durationMs: number;
  contentType?: string | null;
  /** curl -v style debug trace */
  verbose?: string | null;
}

export interface HistoryEntry {
  id: string;
  method: string;
  url: string;
  status?: number | null;
  durationMs?: number | null;
  timestamp: string;
  request: SendRequestInput;
}

export interface Environment {
  id: string;
  name: string;
  variables: KeyValue[];
}

export interface EnvironmentFile {
  environments: Environment[];
  activeId?: string | null;
}

export interface CookieEntry {
  id: string;
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  httpOnly: boolean;
  expires?: string | null;
}

export interface ProxyConfig {
  mode: "system" | "none" | "manual";
  http?: string | null;
  https?: string | null;
  socks?: string | null;
}

export interface ScriptPermissions {
  allowFs: boolean;
  allowNetwork: boolean;
  timeoutMs: number;
}

export interface AppConfig {
  workspacePath?: string | null;
  defaultEngine: string;
  activeEnvId?: string | null;
  proxy: ProxyConfig;
  scriptPermissions: ScriptPermissions;
  theme: "dark" | "light";
}

export interface AssertionResult {
  name: string;
  passed: boolean;
  message: string;
}

export interface ScriptRunResult {
  logs: string[];
  errors: string[];
  assertions: AssertionResult[];
  variables: Record<string, string>;
  request?: {
    method: string;
    url: string;
    headers: KeyValue[];
    query: KeyValue[];
    body: RequestBody;
  } | null;
}

export interface SearchHit {
  collectionId: string;
  collectionName: string;
  itemId: string;
  name: string;
  method: string;
  url: string;
  path: string;
}

export type EditorTab = "params" | "headers" | "body" | "auth" | "scripts" | "settings";
export type ResponseTab = "body" | "headers" | "tests" | "console" | "verbose";
export type BodyView = "pretty" | "raw";
