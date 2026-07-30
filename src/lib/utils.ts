import type {
  AuthConfig,
  HttpMethod,
  HttpRequest,
  KeyValue,
  ScriptBlock,
} from "./types";

export const METHODS: HttpMethod[] = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
];

export const METHOD_COLORS: Record<string, string> = {
  GET: "text-emerald-400",
  POST: "text-amber-400",
  PUT: "text-sky-400",
  PATCH: "text-violet-400",
  DELETE: "text-rose-400",
  HEAD: "text-slate-400",
  OPTIONS: "text-slate-400",
};

export function emptyKeyValue(): KeyValue {
  return { key: "", value: "", enabled: true };
}

export function emptyAuth(): AuthConfig {
  return {
    type: "none",
    token: "",
    username: "",
    password: "",
    key: "",
    value: "",
    addTo: "header",
  };
}

export function emptyScripts(): ScriptBlock {
  return { preRequest: "", postResponse: "", engine: null };
}

export function createEmptyRequest(
  id: string,
  name = "New Request",
): HttpRequest {
  return {
    id,
    name,
    method: "GET",
    url: "https://httpbin.org/get",
    headers: [emptyKeyValue()],
    query: [emptyKeyValue()],
    body: { type: "none", content: "" },
    auth: emptyAuth(),
    config: {
      timeoutMs: 30000,
      maxRedirects: 10,
      followRedirects: true,
    },
    scripts: emptyScripts(),
  };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export function tryPrettyJson(text: string): string | null {
  try {
    const parsed = JSON.parse(text);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return null;
  }
}

export function statusBadgeBg(status: number): string {
  if (status >= 200 && status < 300)
    return "bg-emerald-500/15 text-emerald-400 ring-emerald-500/30";
  if (status >= 300 && status < 400)
    return "bg-sky-500/15 text-sky-400 ring-sky-500/30";
  if (status >= 400 && status < 500)
    return "bg-amber-500/15 text-amber-400 ring-amber-500/30";
  if (status >= 500)
    return "bg-rose-500/15 text-rose-400 ring-rose-500/30";
  return "bg-slate-500/15 text-slate-400 ring-slate-500/30";
}

export function varsFromEnv(
  variables: KeyValue[],
): Record<string, string> {
  const map: Record<string, string> = {};
  for (const v of variables) {
    if (v.enabled && v.key) map[v.key] = v.value;
  }
  return map;
}
