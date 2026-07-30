<script lang="ts">
  import { onMount } from "svelte";
  import { importCurl } from "$lib/api";
  import type { BodyType, EditorTab, HttpRequest } from "$lib/types";
  import {
    emptyAuth,
    emptyKeyValue,
    emptyScripts,
    fieldsToBodyContent,
    looksLikeCurl,
    METHODS,
    parseBodyFields,
  } from "$lib/utils";
  import KeyValueEditor from "./KeyValueEditor.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import type { KeyValue } from "$lib/types";

  interface Props {
    request: HttpRequest;
    sending: boolean;
    dark?: boolean;
    defaultEngine?: string;
    onsend: () => void;
    onchange: (request: HttpRequest) => void;
    oncodegen?: (lang: string) => void;
  }

  let {
    request,
    sending,
    dark = false,
    defaultEngine = "rhai",
    onsend,
    onchange,
    oncodegen,
  }: Props = $props();

  let tab = $state<EditorTab>("params");
  let scriptTab = $state<"pre" | "post">("pre");
  let pasteHint = $state<string | null>(null);

  // Postman-style form-data rows derived from body.content
  let formRows = $derived.by((): KeyValue[] => {
    if (request.body.type === "form" || request.body.type === "multipart") {
      return parseBodyFields(request.body.content);
    }
    return [emptyKeyValue()];
  });

  const bodyTypes: { id: BodyType; label: string }[] = [
    { id: "none", label: "none" },
    { id: "form", label: "form-data" },
    { id: "multipart", label: "multipart" },
    { id: "json", label: "JSON" },
    { id: "raw", label: "raw" },
    { id: "binary", label: "binary" },
  ];

  function patch(partial: Partial<HttpRequest>) {
    onchange({ ...request, ...partial });
  }

  function ensureKvRows(
    list: { key: string; value: string; enabled: boolean }[] | undefined,
  ) {
    const rows = list?.length ? [...list] : [];
    if (rows.length === 0 || rows[rows.length - 1]?.key || rows[rows.length - 1]?.value) {
      rows.push({ key: "", value: "", enabled: true });
    }
    return rows;
  }

  async function onUrlPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text") ?? "";
    if (!looksLikeCurl(text)) return;

    e.preventDefault();
    pasteHint = "Parsing cURL…";
    try {
      const parsed = await importCurl(text);
      const method = (parsed.method?.toUpperCase() ||
        request.method) as HttpRequest["method"];
      // Map auth: backend uses type via camelCase serde
      const auth = {
        ...emptyAuth(),
        ...(parsed.auth ?? {}),
        type: (parsed.auth?.type || "none") as HttpRequest["auth"]["type"],
      };
      onchange({
        ...request,
        name:
          !request.name ||
          request.name === "New Request" ||
          request.name.startsWith("Sample")
            ? parsed.name || "Imported cURL"
            : request.name,
        method,
        url: parsed.url || "",
        headers: ensureKvRows(parsed.headers),
        query: ensureKvRows(parsed.query),
        body: parsed.body ?? { type: "none", content: "" },
        auth,
        config: parsed.config ?? request.config,
        scripts: parsed.scripts ?? request.scripts ?? emptyScripts(),
      });
      if (parsed.body?.type && parsed.body.type !== "none") {
        tab = "body";
      } else if (parsed.headers?.some((h) => h.key)) {
        tab = "headers";
      }
      pasteHint = "✓ Imported from cURL";
      setTimeout(() => {
        pasteHint = null;
      }, 2500);
    } catch (err) {
      console.error(err);
      pasteHint = `cURL parse failed: ${err}`;
      // Do NOT dump the whole curl into the URL field
      setTimeout(() => {
        pasteHint = null;
      }, 5000);
    }
  }

  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        onsend();
      }
    }
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center gap-2 border-b border-app p-3">
    <select
      class="input-field w-[110px] shrink-0 font-mono text-sm font-semibold"
      value={request.method}
      onchange={(e) =>
        patch({ method: e.currentTarget.value as HttpRequest["method"] })}
    >
      {#each METHODS as m}
        <option value={m}>{m}</option>
      {/each}
    </select>
    <input
      class="input-field min-w-0 flex-1 font-mono text-sm"
      placeholder="Enter URL or paste cURL"
      value={request.url}
      oninput={(e) => patch({ url: e.currentTarget.value })}
      onpaste={onUrlPaste}
    />
    <button
      type="button"
      class="btn-primary min-w-[88px]"
      disabled={sending || !request.url.trim()}
      onclick={onsend}
    >
      {sending ? "Sending…" : "Send"}
    </button>
  </div>

  <div class="flex items-center gap-2 border-b border-app px-3 py-2">
    <input
      class="input-ghost min-w-0 flex-1 text-sm font-medium"
      placeholder="Request name"
      value={request.name}
      oninput={(e) => patch({ name: e.currentTarget.value })}
    />
    {#if pasteHint}
      <span class="text-[11px] font-medium text-[#FF6C37]">{pasteHint}</span>
    {/if}
    <div class="flex gap-1">
      <button type="button" class="chip" onclick={() => oncodegen?.("curl")}>cURL</button>
      <button type="button" class="chip" onclick={() => oncodegen?.("fetch")}>JS</button>
      <button type="button" class="chip" onclick={() => oncodegen?.("python")}>Python</button>
    </div>
    <span class="text-[11px] text-neutral-500">⌘/Ctrl + Enter</span>
  </div>

  <div class="flex flex-wrap items-center gap-1 border-b border-app px-3">
    {#each [
      ["params", "Params"],
      ["headers", "Headers"],
      ["body", "Body"],
      ["auth", "Auth"],
      ["scripts", "Scripts"],
      ["settings", "Settings"],
    ] as [id, label]}
      <button
        type="button"
        class="tab-btn {tab === id ? 'tab-active' : ''}"
        onclick={() => (tab = id as EditorTab)}
      >
        {label}
      </button>
    {/each}
  </div>

  <div class="min-h-0 flex-1 overflow-auto p-3">
    {#if tab === "params"}
      <KeyValueEditor
        items={request.query}
        keyPlaceholder="Query param"
        valuePlaceholder="Value"
        onchange={(query) => patch({ query })}
      />
    {:else if tab === "headers"}
      <KeyValueEditor
        items={request.headers}
        keyPlaceholder="Header"
        valuePlaceholder="Value"
        onchange={(headers) => patch({ headers })}
      />
    {:else if tab === "body"}
      <div class="mb-3 flex flex-wrap gap-1">
        {#each bodyTypes as bt}
          <button
            type="button"
            class="chip {request.body.type === bt.id ? 'chip-active' : ''}"
            onclick={() => {
              let content = request.body.content;
              if (bt.id === "json" && (!content || content.startsWith("["))) {
                content = "{\n  \n}";
              } else if (
                (bt.id === "form" || bt.id === "multipart") &&
                (!content || content.startsWith("{"))
              ) {
                content = fieldsToBodyContent([emptyKeyValue()]);
              }
              patch({ body: { ...request.body, type: bt.id, content } });
            }}
          >
            {bt.label}
          </button>
        {/each}
      </div>
      {#if request.body.type === "none"}
        <p class="py-8 text-center text-sm text-neutral-500">This request has no body.</p>
      {:else if request.body.type === "json"}
        <CodeEditor
          value={request.body.content}
          language="json"
          {dark}
          onchange={(content) => patch({ body: { ...request.body, content } })}
        />
      {:else if request.body.type === "form" || request.body.type === "multipart"}
        <p class="mb-2 text-[11px] text-neutral-500">
          {request.body.type === "multipart"
            ? "form-data — Key / Value (like Postman)"
            : "x-www-form-urlencoded — Key / Value"}
        </p>
        <KeyValueEditor
          items={formRows}
          keyPlaceholder="Key"
          valuePlaceholder="Value"
          onchange={(fields) =>
            patch({
              body: {
                ...request.body,
                content: fieldsToBodyContent(fields),
              },
            })}
        />
      {:else}
        <textarea
          class="input-field min-h-[220px] w-full resize-y font-mono text-xs leading-relaxed"
          placeholder={request.body.type === "binary"
            ? "Base64-encoded content"
            : "Request body"}
          value={request.body.content}
          oninput={(e) =>
            patch({ body: { ...request.body, content: e.currentTarget.value } })}
        ></textarea>
      {/if}
    {:else if tab === "auth"}
      <div class="mb-3 flex flex-wrap gap-1">
        {#each ["none", "bearer", "basic", "apikey"] as t}
          <button
            type="button"
            class="chip {request.auth.type === t ? 'chip-active' : ''}"
            onclick={() =>
              patch({ auth: { ...request.auth, type: t as typeof request.auth.type } })}
            >{t}</button
          >
        {/each}
      </div>
      {#if request.auth.type === "bearer"}
        <span class="mb-1 block text-xs text-neutral-500">Token</span>
        <input
          class="input-field w-full font-mono text-xs"
          value={request.auth.token}
          oninput={(e) =>
            patch({ auth: { ...request.auth, token: e.currentTarget.value } })}
        />
      {:else if request.auth.type === "basic"}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <span class="mb-1 block text-xs text-neutral-500">Username</span>
            <input
              class="input-field w-full font-mono text-xs"
              value={request.auth.username}
              oninput={(e) =>
                patch({ auth: { ...request.auth, username: e.currentTarget.value } })}
            />
          </div>
          <div>
            <span class="mb-1 block text-xs text-neutral-500">Password</span>
            <input
              class="input-field w-full font-mono text-xs"
              type="password"
              value={request.auth.password}
              oninput={(e) =>
                patch({ auth: { ...request.auth, password: e.currentTarget.value } })}
            />
          </div>
        </div>
      {:else if request.auth.type === "apikey"}
        <div class="grid grid-cols-3 gap-2">
          <div>
            <span class="mb-1 block text-xs text-neutral-500">Key</span>
            <input
              class="input-field w-full font-mono text-xs"
              value={request.auth.key}
              oninput={(e) =>
                patch({ auth: { ...request.auth, key: e.currentTarget.value } })}
            />
          </div>
          <div>
            <span class="mb-1 block text-xs text-neutral-500">Value</span>
            <input
              class="input-field w-full font-mono text-xs"
              value={request.auth.value}
              oninput={(e) =>
                patch({ auth: { ...request.auth, value: e.currentTarget.value } })}
            />
          </div>
          <div>
            <span class="mb-1 block text-xs text-neutral-500">Add to</span>
            <select
              class="input-field w-full text-xs"
              value={request.auth.addTo}
              onchange={(e) =>
                patch({
                  auth: {
                    ...request.auth,
                    addTo: e.currentTarget.value as "header" | "query",
                  },
                })}
            >
              <option value="header">Header</option>
              <option value="query">Query</option>
            </select>
          </div>
        </div>
      {:else}
        <p class="text-sm text-neutral-500">No authentication.</p>
      {/if}
    {:else if tab === "scripts"}
      <div class="mb-2 flex items-center gap-2">
        <button
          type="button"
          class="tab-btn {scriptTab === 'pre' ? 'tab-active' : ''}"
          onclick={() => (scriptTab = "pre")}>Pre-request</button
        >
        <button
          type="button"
          class="tab-btn {scriptTab === 'post' ? 'tab-active' : ''}"
          onclick={() => (scriptTab = "post")}>Post-response</button
        >
        <select
          class="input-field ml-auto w-36 text-xs"
          value={request.scripts.engine ?? "inherit"}
          onchange={(e) =>
            patch({
              scripts: {
                ...request.scripts,
                engine:
                  e.currentTarget.value === "inherit" ? null : e.currentTarget.value,
              },
            })}
        >
          <option value="inherit">Engine: inherit ({defaultEngine})</option>
          <option value="rhai">Rhai</option>
          <option value="javascript">JavaScript</option>
        </select>
      </div>
      {#if scriptTab === "pre"}
        <CodeEditor
          value={request.scripts.preRequest}
          language="javascript"
          {dark}
          minHeight="240px"
          onchange={(preRequest) =>
            patch({ scripts: { ...request.scripts, preRequest } })}
        />
        <p class="mt-2 text-[11px] text-neutral-500">
          API: req, env.get/set, fs.read/write/append, tools.*, print/console.log
        </p>
      {:else}
        <CodeEditor
          value={request.scripts.postResponse}
          language="javascript"
          {dark}
          minHeight="240px"
          onchange={(postResponse) =>
            patch({ scripts: { ...request.scripts, postResponse } })}
        />
        <p class="mt-2 text-[11px] text-neutral-500">
          API: res, assert_status, assert_body_field, assert_duration_lt, env, fs, tools
        </p>
      {/if}
    {:else if tab === "settings"}
      <div class="grid max-w-md grid-cols-2 gap-3">
        <div>
          <span class="mb-1 block text-xs text-neutral-500">Timeout (ms)</span>
          <input
            class="input-field w-full text-xs"
            type="number"
            value={request.config.timeoutMs ?? 30000}
            oninput={(e) =>
              patch({
                config: {
                  ...request.config,
                  timeoutMs: Number(e.currentTarget.value) || 30000,
                },
              })}
          />
        </div>
        <div>
          <span class="mb-1 block text-xs text-neutral-500">Max redirects</span>
          <input
            class="input-field w-full text-xs"
            type="number"
            value={request.config.maxRedirects ?? 10}
            oninput={(e) =>
              patch({
                config: {
                  ...request.config,
                  maxRedirects: Number(e.currentTarget.value) || 0,
                },
              })}
          />
        </div>
        <label class="col-span-2 flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300">
          <input
            type="checkbox"
            checked={request.config.followRedirects !== false}
            onchange={(e) =>
              patch({
                config: {
                  ...request.config,
                  followRedirects: e.currentTarget.checked,
                },
              })}
          />
          Follow redirects
        </label>
      </div>
    {/if}
  </div>
</div>
