<script lang="ts">
  import { onMount } from "svelte";
  import {
    addRequest,
    appendHistory,
    clearHistory,
    deleteCollection,
    deleteCookie,
    executeScripts,
    generateCode,
    getConfig,
    getWorkspacePath,
    importPostman,
    listCollections,
    loadCookies,
    loadEnvironments,
    loadHistory,
    newId,
    saveCollection,
    saveConfig,
    saveCookies,
    saveEnvironments,
    searchRequests,
    sendHttpRequest,
    setWorkspacePath,
    treeDelete,
    treeRename,
    treeReorder,
  } from "$lib/api";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import RequestEditor from "$lib/components/RequestEditor.svelte";
  import ResponseViewer from "$lib/components/ResponseViewer.svelte";
  import KeyValueEditor from "$lib/components/KeyValueEditor.svelte";
  import type {
    AppConfig,
    AssertionResult,
    Collection,
    CollectionItem,
    CookieEntry,
    EnvironmentFile,
    HistoryEntry,
    HttpRequest,
    HttpResponse,
    SearchHit,
  } from "$lib/types";
  import {
    createEmptyRequest,
    deepClone,
    emptyAuth,
    emptyKeyValue,
    emptyScripts,
    varsFromEnv,
  } from "$lib/utils";

  let collections = $state<Collection[]>([]);
  let history = $state<HistoryEntry[]>([]);
  let workspacePath = $state("");
  let selectedCollectionId = $state<string | null>(null);
  let selectedItemId = $state<string | null>(null);
  let request = $state<HttpRequest | null>(null);
  let response = $state<HttpResponse | null>(null);
  let error = $state<string | null>(null);
  let sending = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let dirty = $state(false);
  let config = $state<AppConfig>({
    defaultEngine: "rhai",
    proxy: { mode: "system" },
    scriptPermissions: { allowFs: true, allowNetwork: false, timeoutMs: 5000 },
    theme: "light",
  });
  let envFile = $state<EnvironmentFile>({ environments: [], activeId: null });
  let cookies = $state<CookieEntry[]>([]);
  let logs = $state<string[]>([]);
  let assertions = $state<AssertionResult[]>([]);
  let showEnv = $state(false);
  let showSettings = $state(false);
  let showImport = $state(false);
  let showCookies = $state(false);
  let showCodegen = $state(false);
  let codegenText = $state("");
  let importText = $state("");
  let searchQ = $state("");
  let searchHits = $state<SearchHit[]>([]);
  let showSearch = $state(false);

  let dark = $derived(config.theme !== "light");
  let activeEnv = $derived(
    envFile.environments.find((e) => e.id === (config.activeEnvId ?? envFile.activeId)) ??
      null,
  );

  onMount(() => {
    void (async () => {
      try {
        config = await getConfig();
        applyTheme(config.theme);
        collections = await listCollections();
        history = await loadHistory();
        workspacePath = await getWorkspacePath();
        envFile = await loadEnvironments();
        cookies = await loadCookies();

        if (envFile.environments.length === 0) {
          const id = await newId();
          envFile = {
            environments: [
              {
                id,
                name: "dev",
                variables: [
                  { key: "base", value: "https://httpbin.org", enabled: true },
                  { key: "", value: "", enabled: true },
                ],
              },
              {
                id: await newId(),
                name: "test",
                variables: [{ key: "", value: "", enabled: true }],
              },
              {
                id: await newId(),
                name: "prod",
                variables: [{ key: "", value: "", enabled: true }],
              },
            ],
            activeId: id,
          };
          await saveEnvironments(envFile);
          config = { ...config, activeEnvId: id };
          await saveConfig(config);
        }

        if (collections.length === 0) {
          await createDefaultCollection();
        } else {
          for (const col of collections) {
            const first = findFirstRequest(col.items);
            if (first) {
              selectItem(col.id, first);
              break;
            }
          }
        }
      } catch (e) {
        error = String(e);
      }
    })();

    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        showSearch = true;
      }
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "l") {
        e.preventDefault();
        showEnv = !showEnv;
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function applyTheme(theme: string) {
    document.documentElement.classList.toggle("light", theme === "light");
    document.documentElement.classList.toggle("dark", theme !== "light");
  }

  function findFirstRequest(items: CollectionItem[]): CollectionItem | null {
    for (const item of items) {
      if (item.type === "request" && item.request) return item;
      if (item.children) {
        const f = findFirstRequest(item.children);
        if (f) return f;
      }
    }
    return null;
  }

  function normalizeRequest(r: HttpRequest): HttpRequest {
    return {
      ...r,
      headers: (r.headers ?? []).filter((h) => h.key?.trim()),
      query: (r.query ?? []).filter((q) => q.key?.trim()),
      auth: r.auth ?? emptyAuth(),
      config: r.config ?? {
        timeoutMs: 30000,
        maxRedirects: 10,
        followRedirects: true,
      },
      scripts: r.scripts ?? emptyScripts(),
    };
  }

  async function createDefaultCollection() {
    const colId = await newId();
    const reqId = await newId();
    const req = createEmptyRequest(reqId, "Sample GET");
    req.url = "{{base}}/get";
    const col: Collection = {
      id: colId,
      name: "My Collection",
      version: "1.0",
      items: [{ id: reqId, type: "request", name: req.name, request: req }],
    };
    await saveCollection(col);
    collections = [col];
    selectItem(colId, col.items[0]);
  }

  function selectItem(collectionId: string, item: CollectionItem) {
    selectedCollectionId = collectionId;
    selectedItemId = item.id;
    if (item.request) {
      // deepClone: Svelte $state proxies throw with structuredClone
      request = deepClone(normalizeRequest(item.request));
    } else {
      request = null;
    }
    response = null;
    error = null;
    logs = [];
    assertions = [];
  }

  function onRequestChange(next: HttpRequest) {
    request = next;
    dirty = true;
    scheduleSave();
  }

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void persistCurrent();
    }, 400);
  }

  function updateItemInTree(
    items: CollectionItem[],
    id: string,
    updater: (item: CollectionItem) => CollectionItem,
  ): CollectionItem[] {
    return items.map((item) => {
      if (item.id === id) return updater(item);
      if (item.children) {
        return { ...item, children: updateItemInTree(item.children, id, updater) };
      }
      return item;
    });
  }

  async function persistCurrent() {
    if (!request || !selectedCollectionId || !selectedItemId) return;
    const colIdx = collections.findIndex((c) => c.id === selectedCollectionId);
    if (colIdx < 0) return;
    const col = deepClone(collections[colIdx]);
    col.items = updateItemInTree(col.items, selectedItemId, (item) => ({
      ...item,
      name: request!.name,
      request: {
        ...request!,
        headers: request!.headers.filter((h) => h.key || h.value),
        query: request!.query.filter((q) => q.key || q.value),
      },
    }));
    try {
      await saveCollection(col);
      collections = collections.map((c, i) => (i === colIdx ? col : c));
      dirty = false;
    } catch (e) {
      console.error("Save failed", e);
    }
  }

  function collectScripts(
    col: Collection,
    itemId: string,
  ): { pre: string[]; post: string[]; engine: string } {
    const pre: string[] = [];
    const post: string[] = [];
    let engine = col.engine ?? config.defaultEngine ?? "rhai";

    if (col.scripts?.preRequest) pre.push(col.scripts.preRequest);
    if (col.scripts?.postResponse) post.push(col.scripts.postResponse);
    if (col.scripts?.engine) engine = col.scripts.engine;

    function walk(items: CollectionItem[], path: CollectionItem[]): boolean {
      for (const item of items) {
        const next = [...path, item];
        if (item.id === itemId) {
          for (const p of next) {
            if (p.type === "folder" && p.scripts) {
              if (p.scripts.preRequest) pre.push(p.scripts.preRequest);
              if (p.scripts.postResponse) post.push(p.scripts.postResponse);
              if (p.scripts.engine) engine = p.scripts.engine;
            }
          }
          if (item.request?.scripts) {
            if (item.request.scripts.preRequest) pre.push(item.request.scripts.preRequest);
            if (item.request.scripts.postResponse)
              post.push(item.request.scripts.postResponse);
            if (item.request.scripts.engine) engine = item.request.scripts.engine;
          }
          return true;
        }
        if (item.children && walk(item.children, next)) return true;
      }
      return false;
    }
    walk(col.items, []);
    return { pre, post, engine };
  }

  async function handleSend() {
    if (!request || sending) return;
    await persistCurrent();

    sending = true;
    error = null;
    response = null;
    logs = [];
    assertions = [];

    const variables = activeEnv ? varsFromEnv(activeEnv.variables) : {};
    let working = deepClone(request);

    const col = collections.find((c) => c.id === selectedCollectionId);
    const scriptMeta = col
      ? collectScripts(col, selectedItemId ?? request.id)
      : {
          pre: [request.scripts.preRequest],
          post: [request.scripts.postResponse],
          engine: request.scripts.engine ?? config.defaultEngine,
        };

    try {
      // Pre scripts
      if (scriptMeta.pre.some((s) => s.trim())) {
        const preRes = await executeScripts({
          engine: scriptMeta.engine,
          preScripts: scriptMeta.pre,
          postScripts: [],
          request: {
            method: working.method,
            url: working.url,
            headers: working.headers,
            query: working.query,
            body: working.body,
          },
          response: null,
          variables,
          permissions: config.scriptPermissions,
          fsRoot: workspacePath,
          phase: "pre",
        });
        logs = [...logs, ...preRes.logs];
        if (preRes.errors.length) {
          error = preRes.errors.join("\n");
          sending = false;
          return;
        }
        if (preRes.request) {
          working = {
            ...working,
            method: preRes.request.method as HttpRequest["method"],
            url: preRes.request.url,
            headers: preRes.request.headers?.length
              ? preRes.request.headers
              : working.headers,
            query: preRes.request.query?.length ? preRes.request.query : working.query,
            body: preRes.request.body ?? working.body,
          };
        }
        Object.assign(variables, preRes.variables);
      }

      const payload = {
        method: working.method,
        url: working.url,
        headers: working.headers.filter((h) => h.enabled && h.key),
        query: working.query.filter((q) => q.enabled && q.key),
        body: working.body,
        auth: working.auth,
        config: working.config,
        variables,
        proxy: config.proxy,
      };

      const res = await sendHttpRequest(payload);
      response = res;

      // Post scripts
      if (scriptMeta.post.some((s) => s.trim())) {
        const postRes = await executeScripts({
          engine: scriptMeta.engine,
          preScripts: [],
          postScripts: scriptMeta.post,
          request: {
            method: working.method,
            url: working.url,
            headers: working.headers,
            query: working.query,
            body: working.body,
          },
          response: res,
          variables,
          permissions: config.scriptPermissions,
          fsRoot: workspacePath,
          phase: "post",
        });
        logs = [...logs, ...postRes.logs];
        assertions = postRes.assertions;
        if (postRes.errors.length) {
          logs = [...logs, ...postRes.errors.map((e) => `ERROR: ${e}`)];
        }
        // Persist env vars mutated by scripts into active env
        if (activeEnv && Object.keys(postRes.variables).length) {
          const updated = {
            ...activeEnv,
            variables: [
              ...Object.entries(postRes.variables).map(([key, value]) => ({
                key,
                value,
                enabled: true,
              })),
              emptyKeyValue(),
            ],
          };
          envFile = {
            ...envFile,
            environments: envFile.environments.map((e) =>
              e.id === activeEnv!.id ? updated : e,
            ),
          };
          await saveEnvironments(envFile);
        }
      }

      cookies = await loadCookies();

      const entry: HistoryEntry = {
        id: await newId(),
        method: working.method,
        url: working.url,
        status: res.status,
        durationMs: res.durationMs,
        timestamp: new Date().toISOString(),
        request: payload,
      };
      history = await appendHistory(entry);
    } catch (e) {
      error = typeof e === "string" ? e : String(e);
      try {
        history = await appendHistory({
          id: await newId(),
          method: working.method,
          url: working.url,
          status: null,
          durationMs: null,
          timestamp: new Date().toISOString(),
          request: {
            method: working.method,
            url: working.url,
            headers: working.headers,
            query: working.query,
            body: working.body,
          },
        });
      } catch {
        /* ignore */
      }
    } finally {
      sending = false;
    }
  }

  async function handleNewCollection() {
    const colId = await newId();
    const reqId = await newId();
    const req = createEmptyRequest(reqId, "New Request");
    const col: Collection = {
      id: colId,
      name: `Collection ${collections.length + 1}`,
      version: "1.0",
      items: [{ id: reqId, type: "request", name: req.name, request: req }],
    };
    await saveCollection(col);
    collections = [...collections, col];
    selectItem(colId, col.items[0]);
  }

  function findItemById(
    items: CollectionItem[],
    id: string,
  ): CollectionItem | null {
    for (const it of items) {
      if (it.id === id) return it;
      if (it.children) {
        const f = findItemById(it.children, id);
        if (f) return f;
      }
    }
    return null;
  }

  async function handleNewRequest(collectionId: string, parentId?: string | null) {
    try {
      error = null;
      // Single backend path — mutate + persist + return updated tree
      const result = await addRequest(
        collectionId,
        parentId ?? null,
        "New Request",
      );
      collections = collections.map((c) =>
        c.id === collectionId ? result.collection : c,
      );
      // Expand collection so the new item is visible
      const item = findItemById(result.collection.items, result.requestId);
      if (item) {
        selectItem(collectionId, item);
      } else {
        // fallback: re-list from disk
        collections = await listCollections();
        const col = collections.find((c) => c.id === collectionId);
        const found = col
          ? findItemById(col.items, result.requestId)
          : null;
        if (found) selectItem(collectionId, found);
        else error = `Created request ${result.requestId} but UI could not select it`;
      }
    } catch (e) {
      error = `New request failed: ${String(e)}`;
      console.error("handleNewRequest", e);
    }
  }

  async function handleNewFolder(collectionId: string) {
    try {
      const folderId = await newId();
      const colIdx = collections.findIndex((c) => c.id === collectionId);
      if (colIdx < 0) {
        error = `Collection not found: ${collectionId}`;
        return;
      }
      const col = deepClone(collections[colIdx]);
      col.items = [
        ...col.items,
        {
          id: folderId,
          type: "folder",
          name: "New Folder",
          children: [],
        },
      ];
      await saveCollection(col);
      collections = collections.map((c, i) => (i === colIdx ? col : c));
      error = null;
    } catch (e) {
      error = `New folder failed: ${e}`;
      console.error(e);
    }
  }

  async function handleDeleteCollection(id: string) {
    await deleteCollection(id);
    collections = collections.filter((c) => c.id !== id);
    if (selectedCollectionId === id) {
      selectedCollectionId = null;
      selectedItemId = null;
      request = null;
    }
  }

  async function handleRename(collectionId: string, itemId: string, name: string) {
    const col = collections.find((c) => c.id === collectionId);
    if (!col) return;
    const updated = await treeRename(col, itemId, name);
    collections = collections.map((c) => (c.id === collectionId ? updated : c));
    if (selectedItemId === itemId && request) {
      request = { ...request, name };
    }
  }

  async function handleDeleteItem(collectionId: string, itemId: string) {
    const col = collections.find((c) => c.id === collectionId);
    if (!col) return;
    const updated = await treeDelete(col, itemId);
    collections = collections.map((c) => (c.id === collectionId ? updated : c));
    if (selectedItemId === itemId) {
      request = null;
      selectedItemId = null;
    }
  }

  async function handleReorder(
    collectionId: string,
    parentId: string | null,
    itemId: string,
    toIndex: number,
  ) {
    const col = collections.find((c) => c.id === collectionId);
    if (!col) return;
    const updated = await treeReorder(col, parentId, itemId, toIndex);
    collections = collections.map((c) => (c.id === collectionId ? updated : c));
  }

  function handleSelectHistory(entry: HistoryEntry) {
    const r = entry.request;
    request = normalizeRequest({
      id: request?.id ?? entry.id,
      name: request?.name ?? `${entry.method} ${entry.url}`,
      method: (r.method?.toUpperCase() as HttpRequest["method"]) || "GET",
      url: r.url,
      headers: (r.headers ?? []).filter((h) => h.key?.trim()),
      query: (r.query ?? []).filter((q) => q.key?.trim()),
      body: r.body ?? { type: "none", content: "" },
      auth: (r as { auth?: HttpRequest["auth"] }).auth ?? emptyAuth(),
      config: (r as { config?: HttpRequest["config"] }).config ?? {
        timeoutMs: 30000,
        maxRedirects: 10,
        followRedirects: true,
      },
      scripts: emptyScripts(),
    });
    selectedItemId = null;
    response = null;
    error = null;
  }

  async function handleClearHistory() {
    await clearHistory();
    history = [];
  }

  async function switchEnv(id: string) {
    envFile = { ...envFile, activeId: id };
    config = { ...config, activeEnvId: id };
    await saveEnvironments(envFile);
    await saveConfig(config);
  }

  async function persistEnv() {
    await saveEnvironments(envFile);
  }

  async function toggleTheme() {
    const theme = config.theme === "light" ? "dark" : "light";
    config = { ...config, theme };
    applyTheme(theme);
    await saveConfig(config);
  }

  async function persistSettings() {
    await saveConfig(config);
    if (config.workspacePath !== undefined) {
      workspacePath = await setWorkspacePath(config.workspacePath ?? null);
    }
  }

  async function doImport() {
    try {
      // cURL is imported by pasting into the URL bar — this dialog is Postman only
      const col = await importPostman(importText);
      await saveCollection(col);
      collections = [...collections, col];
      showImport = false;
      importText = "";
      error = null;
    } catch (e) {
      error = `Import failed: ${e}`;
    }
  }

  async function doCodegen(lang: string) {
    if (!request) return;
    codegenText = await generateCode(
      {
        method: request.method,
        url: request.url,
        headers: request.headers,
        query: request.query,
        body: request.body,
        auth: request.auth,
      },
      lang,
    );
    showCodegen = true;
  }

  async function doSearch() {
    searchHits = await searchRequests(searchQ);
  }

  function openSearchHit(hit: SearchHit) {
    const col = collections.find((c) => c.id === hit.collectionId);
    if (!col) return;
    function find(items: CollectionItem[]): CollectionItem | null {
      for (const it of items) {
        if (it.id === hit.itemId) return it;
        if (it.children) {
          const f = find(it.children);
          if (f) return f;
        }
      }
      return null;
    }
    const item = find(col.items);
    if (item) selectItem(col.id, item);
    showSearch = false;
  }
</script>

<div
  class="flex h-screen w-screen overflow-hidden bg-white text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100"
>
  <Sidebar
    {collections}
    {history}
    selectedId={selectedItemId}
    {workspacePath}
    onselect={selectItem}
    addRequest={handleNewRequest}
    addFolder={handleNewFolder}
    addCollection={handleNewCollection}
    removeCollection={handleDeleteCollection}
    onrename={handleRename}
    ondeleteItem={handleDeleteItem}
    onreorder={handleReorder}
    onselectHistory={handleSelectHistory}
    onclearHistory={handleClearHistory}
  />

  <main class="flex min-w-0 flex-1 flex-col">
    {#if error}
      <div
        class="flex shrink-0 items-center gap-2 border-b border-rose-200 bg-rose-50 px-3 py-1.5 text-xs text-rose-700 dark:border-rose-900 dark:bg-rose-950/50 dark:text-rose-300"
      >
        <span class="min-w-0 flex-1 break-all">{error}</span>
        <button type="button" class="icon-btn shrink-0" onclick={() => (error = null)}
          >×</button
        >
      </div>
    {/if}
    <header
      class="flex h-11 shrink-0 items-center gap-2 border-b border-app px-3"
    >
      <div class="flex items-center gap-2">
        <div
          class="flex h-6 w-6 items-center justify-center rounded-md bg-[#FF6C37]/15 text-xs font-bold text-[#FF6C37]"
        >
          F
        </div>
        <span class="text-sm font-semibold tracking-tight">Forge</span>
      </div>

      <select
        class="input-field ml-3 w-32 text-xs"
        value={config.activeEnvId ?? envFile.activeId ?? ""}
        onchange={(e) => switchEnv(e.currentTarget.value)}
      >
        {#each envFile.environments as env}
          <option value={env.id}>{env.name}</option>
        {/each}
      </select>
      <button type="button" class="chip" onclick={() => (showEnv = true)}>Env</button>

      <div class="relative ml-2 min-w-0 flex-1 max-w-md">
        <input
          class="input-field w-full text-xs"
          placeholder="Search requests… (⌘K)"
          bind:value={searchQ}
          oninput={() => doSearch()}
          onfocus={() => (showSearch = true)}
        />
        {#if showSearch && searchHits.length > 0}
          <div
            class="absolute left-0 right-0 top-full z-20 mt-1 max-h-64 overflow-auto rounded-md border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 shadow-xl"
          >
            {#each searchHits as hit}
              <button
                type="button"
                class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs hover:bg-neutral-100 dark:bg-neutral-800"
                onclick={() => openSearchHit(hit)}
              >
                <span class="font-mono text-[10px] text-emerald-400">{hit.method}</span>
                <span class="truncate">{hit.name}</span>
                <span class="ml-auto truncate text-neutral-500">{hit.url}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <button type="button" class="chip" onclick={() => (showImport = true)}
        >Import Postman</button
      >
      <button type="button" class="chip" onclick={() => (showCookies = true)}
        >Cookies</button
      >
      <button type="button" class="chip" onclick={() => (showSettings = true)}
        >Settings</button
      >
      <button type="button" class="chip" onclick={toggleTheme}
        >{dark ? "Light" : "Dark"}</button
      >

      <div class="ml-auto text-xs text-neutral-500">
        {#if dirty}
          <span class="text-amber-400/80">Saving…</span>
        {:else}
          <span>Local · offline</span>
        {/if}
      </div>
    </header>

    {#if request}
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="min-h-0 flex-[1.1]">
          <RequestEditor
            {request}
            {sending}
            {dark}
            defaultEngine={config.defaultEngine}
            onsend={handleSend}
            onchange={onRequestChange}
            oncodegen={doCodegen}
          />
        </div>
        <div class="min-h-0 flex-1">
          <ResponseViewer {response} {error} {sending} {logs} {assertions} {dark} />
        </div>
      </div>
    {:else}
      <div
        class="flex flex-1 flex-col items-center justify-center gap-3 text-neutral-500"
      >
        <p class="text-lg font-medium">No request selected</p>
        <button type="button" class="btn-primary" onclick={handleNewCollection}
          >New Collection</button
        >
      </div>
    {/if}
  </main>
</div>

{#if showEnv}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div
      class="max-h-[80vh] w-full max-w-xl overflow-auto rounded-xl border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-4 shadow-2xl"
    >
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold">Environments</h2>
        <button type="button" class="icon-btn" onclick={() => (showEnv = false)}>×</button>
      </div>
      <div class="mb-3 flex gap-2">
        {#each envFile.environments as env}
          <button
            type="button"
            class="chip {env.id === (config.activeEnvId ?? envFile.activeId)
              ? 'chip-active'
              : ''}"
            onclick={() => switchEnv(env.id)}>{env.name}</button
          >
        {/each}
        <button
          type="button"
          class="chip"
          onclick={async () => {
            const id = await newId();
            envFile = {
              ...envFile,
              environments: [
                ...envFile.environments,
                { id, name: `env${envFile.environments.length + 1}`, variables: [emptyKeyValue()] },
              ],
            };
            await persistEnv();
          }}>+ Env</button
        >
      </div>
      {#if activeEnv}
        <input
          class="input-field mb-2 w-full text-sm"
          value={activeEnv.name}
          oninput={(e) => {
            const name = e.currentTarget.value;
            envFile = {
              ...envFile,
              environments: envFile.environments.map((en) =>
                en.id === activeEnv!.id ? { ...en, name } : en,
              ),
            };
          }}
          onchange={persistEnv}
        />
        <KeyValueEditor
          items={activeEnv.variables}
          keyPlaceholder="Variable"
          valuePlaceholder="Value"
          onchange={(variables) => {
            envFile = {
              ...envFile,
              environments: envFile.environments.map((en) =>
                en.id === activeEnv!.id ? { ...en, variables } : en,
              ),
            };
            void persistEnv();
          }}
        />
      {/if}
    </div>
  </div>
{/if}

{#if showSettings}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div
      class="w-full max-w-lg rounded-xl border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-4 shadow-2xl"
    >
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold">Settings</h2>
        <button type="button" class="icon-btn" onclick={() => (showSettings = false)}
          >×</button
        >
      </div>
      <div class="space-y-3 text-xs">
        <div>
          <label class="mb-1 block text-neutral-500">Default script engine</label>
          <select
            class="input-field w-full"
            value={config.defaultEngine}
            onchange={(e) => (config = { ...config, defaultEngine: e.currentTarget.value })}
          >
            <option value="rhai">Rhai</option>
            <option value="javascript">JavaScript</option>
          </select>
        </div>
        <div>
          <label class="mb-1 block text-neutral-500">Proxy mode</label>
          <select
            class="input-field w-full"
            value={config.proxy.mode}
            onchange={(e) =>
              (config = {
                ...config,
                proxy: {
                  ...config.proxy,
                  mode: e.currentTarget.value as AppConfig["proxy"]["mode"],
                },
              })}
          >
            <option value="system">System</option>
            <option value="none">None</option>
            <option value="manual">Manual</option>
          </select>
        </div>
        {#if config.proxy.mode === "manual"}
          <input
            class="input-field w-full"
            placeholder="HTTP proxy e.g. http://127.0.0.1:7890"
            value={config.proxy.http ?? ""}
            oninput={(e) =>
              (config = {
                ...config,
                proxy: { ...config.proxy, http: e.currentTarget.value },
              })}
          />
          <input
            class="input-field w-full"
            placeholder="HTTPS proxy"
            value={config.proxy.https ?? ""}
            oninput={(e) =>
              (config = {
                ...config,
                proxy: { ...config.proxy, https: e.currentTarget.value },
              })}
          />
          <input
            class="input-field w-full"
            placeholder="SOCKS proxy e.g. socks5://127.0.0.1:1080"
            value={config.proxy.socks ?? ""}
            oninput={(e) =>
              (config = {
                ...config,
                proxy: { ...config.proxy, socks: e.currentTarget.value },
              })}
          />
        {/if}
        <label class="flex items-center gap-2">
          <input
            type="checkbox"
            checked={config.scriptPermissions.allowFs}
            onchange={(e) =>
              (config = {
                ...config,
                scriptPermissions: {
                  ...config.scriptPermissions,
                  allowFs: e.currentTarget.checked,
                },
              })}
          />
          Allow script filesystem access
        </label>
        <label class="flex items-center gap-2">
          <input
            type="checkbox"
            checked={config.scriptPermissions.allowNetwork}
            onchange={(e) =>
              (config = {
                ...config,
                scriptPermissions: {
                  ...config.scriptPermissions,
                  allowNetwork: e.currentTarget.checked,
                },
              })}
          />
          Allow script network (reserved)
        </label>
        <div>
          <label class="mb-1 block text-neutral-500">Script timeout (ms)</label>
          <input
            class="input-field w-full"
            type="number"
            value={config.scriptPermissions.timeoutMs}
            oninput={(e) =>
              (config = {
                ...config,
                scriptPermissions: {
                  ...config.scriptPermissions,
                  timeoutMs: Number(e.currentTarget.value) || 5000,
                },
              })}
          />
        </div>
        <div>
          <label class="mb-1 block text-neutral-500">Workspace path</label>
          <input
            class="input-field w-full font-mono"
            value={config.workspacePath ?? workspacePath}
            oninput={(e) =>
              (config = { ...config, workspacePath: e.currentTarget.value || null })}
          />
        </div>
        <button
          type="button"
          class="btn-primary"
          onclick={async () => {
            await persistSettings();
            showSettings = false;
          }}>Save</button
        >
      </div>
    </div>
  </div>
{/if}

{#if showImport}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div
      class="w-full max-w-xl rounded-xl border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-4 shadow-2xl"
    >
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold">Import Postman Collection</h2>
        <button type="button" class="icon-btn" onclick={() => (showImport = false)}
          >×</button
        >
      </div>
      <p class="mb-2 text-xs text-neutral-500">
        Paste Postman Collection v2.1 JSON. For cURL, paste directly into the URL bar
        instead.
      </p>
      <textarea
        class="input-field mb-3 min-h-[200px] w-full font-mono text-xs"
        placeholder="Paste Postman Collection JSON…"
        bind:value={importText}
      ></textarea>
      <button type="button" class="btn-primary" onclick={doImport}>Import</button>
    </div>
  </div>
{/if}

{#if showCookies}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div
      class="max-h-[80vh] w-full max-w-2xl overflow-auto rounded-xl border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-4 shadow-2xl"
    >
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold">Cookies</h2>
        <button type="button" class="icon-btn" onclick={() => (showCookies = false)}
          >×</button
        >
      </div>
      {#if cookies.length === 0}
        <p class="text-sm text-neutral-500">No cookies stored.</p>
      {:else}
        <table class="w-full text-left text-xs">
          <thead class="text-neutral-500">
            <tr>
              <th class="pb-2">Name</th>
              <th class="pb-2">Value</th>
              <th class="pb-2">Domain</th>
              <th class="pb-2">Path</th>
              <th class="pb-2"></th>
            </tr>
          </thead>
          <tbody>
            {#each cookies as c}
              <tr class="border-t border-app">
                <td class="py-1 font-mono">{c.name}</td>
                <td class="py-1 font-mono">
                  <input
                    class="input-field w-full py-0.5"
                    value={c.value}
                    onchange={async (e) => {
                      cookies = cookies.map((x) =>
                        x.id === c.id ? { ...x, value: e.currentTarget.value } : x,
                      );
                      await saveCookies(cookies);
                    }}
                  />
                </td>
                <td class="py-1 font-mono">{c.domain}</td>
                <td class="py-1 font-mono">{c.path}</td>
                <td class="py-1">
                  <button
                    type="button"
                    class="icon-btn text-rose-400"
                    onclick={async () => {
                      cookies = await deleteCookie(c.id);
                    }}>×</button
                  >
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
{/if}

{#if showCodegen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div
      class="w-full max-w-xl rounded-xl border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-4 shadow-2xl"
    >
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold">Code generation</h2>
        <button type="button" class="icon-btn" onclick={() => (showCodegen = false)}
          >×</button
        >
      </div>
      <pre
        class="max-h-96 overflow-auto rounded-md border border-app bg-white dark:bg-neutral-950 p-3 font-mono text-xs whitespace-pre-wrap"
        >{codegenText}</pre
      >
    </div>
  </div>
{/if}
