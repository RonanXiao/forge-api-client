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
  GET: "text-emerald-600 dark:text-emerald-400",
  POST: "text-[#FF6C37]",
  PUT: "text-sky-600 dark:text-sky-400",
  PATCH: "text-violet-600 dark:text-violet-400",
  DELETE: "text-rose-600 dark:text-rose-400",
  HEAD: "text-neutral-500",
  OPTIONS: "text-neutral-500",
};

export function emptyKeyValue(): KeyValue {
  return { key: "", value: "", enabled: true };
}

/** Parse form/multipart body.content → key/value rows (JSON array or legacy key=value). */
export function parseBodyFields(content: string): KeyValue[] {
  const trimmed = content.trim();
  if (!trimmed) return [emptyKeyValue()];
  if (trimmed.startsWith("[")) {
    try {
      const arr = JSON.parse(trimmed) as Array<{
        key?: string;
        value?: string;
        enabled?: boolean;
      }>;
      if (Array.isArray(arr)) {
        const rows = arr.map((r) => ({
          key: r.key ?? "",
          value: String(r.value ?? ""),
          enabled: r.enabled !== false,
        }));
        if (rows.length === 0 || rows[rows.length - 1].key || rows[rows.length - 1].value) {
          rows.push(emptyKeyValue());
        }
        return rows;
      }
    } catch {
      /* fall through */
    }
  }
  const rows: KeyValue[] = [];
  for (const line of content.split("\n")) {
    const t = line.trim();
    if (!t) continue;
    const i = t.indexOf("=");
    if (i >= 0) {
      rows.push({ key: t.slice(0, i), value: t.slice(i + 1), enabled: true });
    }
  }
  rows.push(emptyKeyValue());
  return rows.length > 1 ? rows : [emptyKeyValue()];
}

/** Serialize key/value rows to body.content JSON storage (drop empty keys). */
export function fieldsToBodyContent(fields: KeyValue[]): string {
  const cleaned = fields.filter((f) => f.key.trim().length > 0);
  return JSON.stringify(
    cleaned.map((f) => ({ key: f.key, value: f.value, enabled: f.enabled })),
    null,
    2,
  );
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
    return "bg-emerald-500/15 text-emerald-700 ring-emerald-500/30 dark:text-emerald-400";
  if (status >= 300 && status < 400)
    return "bg-sky-500/15 text-sky-700 ring-sky-500/30 dark:text-sky-400";
  if (status >= 400 && status < 500)
    return "bg-amber-500/15 text-amber-700 ring-amber-500/30 dark:text-amber-400";
  if (status >= 500)
    return "bg-rose-500/15 text-rose-700 ring-rose-500/30 dark:text-rose-400";
  return "bg-neutral-500/15 text-neutral-600 ring-neutral-500/30 dark:text-neutral-400";
}

/**
 * Deep clone plain data. Prefer this over structuredClone for Svelte 5 $state
 * proxies (structuredClone throws DataCloneError on Proxies).
 */
export function deepClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** Detect pasted cURL command (single or multi-line, optional leading whitespace). */
export function looksLikeCurl(text: string): boolean {
  const t = text.trim();
  if (!t) return false;
  // Allow "curl ..." or "CURL ..." after optional shell noise
  if (/^curl(\s|$)/i.test(t)) return true;
  // Multi-line export sometimes starts with comment then curl
  if (/\n\s*curl\s/i.test(t) && t.toLowerCase().includes("curl ")) return true;
  return false;
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
