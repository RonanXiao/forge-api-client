<script lang="ts">
  import type {
    Collection,
    CollectionItem,
    Environment,
    HistoryEntry,
  } from "$lib/types";
  import { METHOD_COLORS } from "$lib/utils";

  export type SidebarPanel = "collections" | "history" | "env";

  interface Props {
    collections: Collection[];
    history: HistoryEntry[];
    environments?: Environment[];
    activeEnvId?: string | null;
    selectedId: string | null;
    workspacePath: string;
    /** Controlled panel tab — Collections / History / Env */
    panel?: SidebarPanel;
    onpanel: (panel: SidebarPanel) => void;
    onselect: (collectionId: string, item: CollectionItem) => void;
    /** Avoid on* prop names — Svelte can treat them oddly as event handlers */
    addRequest: (collectionId: string, parentId?: string | null) => void;
    addFolder: (collectionId: string) => void;
    addCollection: () => void;
    removeCollection: (id: string) => void;
    onrename: (collectionId: string, itemId: string, name: string) => void;
    ondeleteItem: (collectionId: string, itemId: string) => void;
    onreorder: (
      collectionId: string,
      parentId: string | null,
      itemId: string,
      toIndex: number,
    ) => void;
    onselectHistory: (entry: HistoryEntry) => void;
    onclearHistory: () => void;
    onselectEnv?: (id: string) => void;
    onaddEnv?: () => void;
    ondeleteEnv?: (id: string) => void;
  }

  let {
    collections,
    history,
    environments = [],
    activeEnvId = null,
    selectedId,
    workspacePath,
    panel = "collections",
    onpanel,
    onselect,
    addRequest,
    addFolder,
    addCollection,
    removeCollection,
    onrename,
    ondeleteItem,
    onreorder,
    onselectHistory,
    onclearHistory,
    onselectEnv,
    onaddEnv,
    ondeleteEnv,
  }: Props = $props();

  let expanded = $state<Record<string, boolean>>({});
  let editingId = $state<string | null>(null);
  let editName = $state("");
  let dragId = $state<string | null>(null);

  function setPanel(p: SidebarPanel) {
    onpanel(p);
  }

  function toggle(id: string) {
    expanded = { ...expanded, [id]: !expanded[id] };
  }

  function isExpanded(id: string) {
    return expanded[id] !== false;
  }

  function startRename(id: string, name: string) {
    editingId = id;
    editName = name;
  }

  function commitRename(collectionId: string, itemId: string) {
    if (editName.trim()) onrename(collectionId, itemId, editName.trim());
    editingId = null;
  }
</script>

<aside
  class="flex h-full w-64 shrink-0 flex-col border-r border-app bg-neutral-50 dark:bg-neutral-950"
>
  <div class="flex items-center gap-0.5 border-b border-app p-1.5">
    <button
      type="button"
      class="tab-btn flex-1 px-1 text-[11px] {panel === 'collections' ? 'tab-active' : ''}"
      onclick={() => setPanel("collections")}
    >
      Collections
    </button>
    <button
      type="button"
      class="tab-btn flex-1 px-1 text-[11px] {panel === 'history' ? 'tab-active' : ''}"
      onclick={() => setPanel("history")}
    >
      History
    </button>
    <button
      type="button"
      class="tab-btn flex-1 px-1 text-[11px] {panel === 'env' ? 'tab-active' : ''}"
      onclick={() => setPanel("env")}
    >
      Env
    </button>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if panel === "collections"}
      <div class="mb-2 flex items-center justify-between px-1">
        <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500"
          >Local</span
        >
        <button type="button" class="icon-btn text-xs" onclick={() => addCollection()}
          >+ New</button
        >
      </div>

      {#if collections.length === 0}
        <p class="px-2 py-6 text-center text-xs text-neutral-500">
          No collections yet.
        </p>
      {/if}

      {#each collections as col (col.id)}
        <div class="mb-1">
          <div
            class="group flex items-center gap-1 rounded-md px-1.5 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800/60"
          >
            <button
              type="button"
              class="text-neutral-500 hover:text-neutral-700 dark:text-neutral-300"
              onclick={() => toggle(col.id)}
            >
              <span class="inline-block w-3 text-[10px]"
                >{isExpanded(col.id) ? "▼" : "▶"}</span
              >
            </button>
            <span class="min-w-0 flex-1 truncate text-sm font-medium text-neutral-800 dark:text-neutral-200"
              >{col.name}</span
            >
            <button
              type="button"
              class="icon-btn inline-flex shrink-0 px-1.5 text-[15px] font-bold text-[#FF6C37]"
              title="New request"
              onclick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                addRequest(col.id);
              }}>+</button
            >
            <button
              type="button"
              class="icon-btn inline-flex shrink-0 text-[11px]"
              title="New folder"
              onclick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                addFolder(col.id);
              }}>📁</button
            >
            <button
              type="button"
              class="icon-btn inline-flex shrink-0 text-[11px] text-rose-500"
              title="Delete collection"
              onclick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                removeCollection(col.id);
              }}>×</button
            >
          </div>

          {#if isExpanded(col.id)}
            <ul class="ml-3 border-l border-app pl-1">
              {#each col.items as item, idx (item.id)}
                {@render treeItem(col.id, item, null, idx, col.items.length)}
              {/each}
            </ul>
          {/if}
        </div>
      {/each}
    {:else if panel === "history"}
      <div class="mb-2 flex items-center justify-between px-1">
        <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500"
          >Recent</span
        >
        {#if history.length > 0}
          <button
            type="button"
            class="icon-btn text-[11px] text-rose-400/80"
            onclick={onclearHistory}>Clear</button
          >
        {/if}
      </div>
      {#each history as entry (entry.id)}
        <button
          type="button"
          class="mb-0.5 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-neutral-100 dark:bg-neutral-800/50"
          onclick={() => onselectHistory(entry)}
        >
          <span
            class="w-12 shrink-0 font-mono text-[10px] font-bold {METHOD_COLORS[
              entry.method
            ] ?? 'text-neutral-500 dark:text-neutral-400'}">{entry.method}</span
          >
          <div class="min-w-0 flex-1">
            <div class="truncate text-xs text-neutral-700 dark:text-neutral-300">{entry.url}</div>
            <div class="text-[10px] text-neutral-500">
              {entry.status ?? "—"} · {entry.durationMs ?? "—"}ms
            </div>
          </div>
        </button>
      {/each}
    {:else}
      <!-- Env list -->
      <div class="mb-2 flex items-center justify-between px-1">
        <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500"
          >Environments</span
        >
        <button
          type="button"
          class="icon-btn text-xs font-bold text-[#FF6C37]"
          title="New environment"
          onclick={() => onaddEnv?.()}>+</button
        >
      </div>
      {#if environments.length === 0}
        <p class="px-2 py-6 text-center text-xs text-neutral-500">
          No environments yet.
        </p>
      {/if}
      {#each environments as env (env.id)}
        <div
          class="group mb-0.5 flex items-center gap-0.5 rounded-md
            {env.id === activeEnvId
            ? 'bg-[#FF6C37]/12'
            : 'hover:bg-neutral-100 dark:hover:bg-neutral-800/60'}"
        >
          <button
            type="button"
            class="min-w-0 flex-1 truncate px-2.5 py-2 text-left text-sm
              {env.id === activeEnvId
              ? 'font-medium text-neutral-900 dark:text-white'
              : 'text-neutral-700 dark:text-neutral-300'}"
            onclick={() => onselectEnv?.(env.id)}
          >
            <span class="flex items-center gap-2">
              <span
                class="h-1.5 w-1.5 shrink-0 rounded-full
                  {env.id === activeEnvId ? 'bg-[#FF6C37]' : 'bg-neutral-300 dark:bg-neutral-600'}"
              ></span>
              <span class="truncate">{env.name || "Untitled"}</span>
            </span>
            <span class="mt-0.5 block pl-3.5 text-[10px] text-neutral-400">
              {env.variables.filter((v) => v.key?.trim()).length} variables
            </span>
          </button>
          <button
            type="button"
            class="mr-1 hidden h-7 w-7 shrink-0 items-center justify-center rounded text-neutral-400 group-hover:inline-flex hover:bg-rose-50 hover:text-rose-500 dark:hover:bg-rose-950/40"
            title="Delete environment"
            onclick={(e) => {
              e.stopPropagation();
              ondeleteEnv?.(env.id);
            }}>×</button
          >
        </div>
      {/each}
    {/if}
  </div>

  <div
    class="border-t border-app px-3 py-2 text-[10px] text-neutral-500"
    title={workspacePath}
  >
    <div class="truncate">📁 {workspacePath || "—"}</div>
  </div>
</aside>

{#snippet treeItem(
  collectionId: string,
  item: CollectionItem,
  parentId: string | null,
  index: number,
  siblingCount: number,
)}
  <li
    class="group"
    draggable="true"
    ondragstart={() => (dragId = item.id)}
    ondragover={(e) => e.preventDefault()}
    ondrop={() => {
      if (dragId && dragId !== item.id) {
        onreorder(collectionId, parentId, dragId, index);
      }
      dragId = null;
    }}
  >
    {#if item.type === "folder"}
      <div class="flex items-center gap-1 rounded-md px-2 py-1 hover:bg-neutral-100 dark:bg-neutral-800/50">
        <button type="button" class="text-[10px] text-neutral-500" onclick={() => toggle(item.id)}>
          {isExpanded(item.id) ? "▼" : "▶"}
        </button>
        {#if editingId === item.id}
          <input
            class="input-field flex-1 py-0.5 text-xs"
            bind:value={editName}
            onkeydown={(e) => e.key === "Enter" && commitRename(collectionId, item.id)}
            onblur={() => commitRename(collectionId, item.id)}
          />
        {:else}
          <span class="min-w-0 flex-1 truncate text-xs text-neutral-700 dark:text-neutral-300">📁 {item.name}</span>
          <button
            type="button"
            class="icon-btn inline-flex shrink-0 px-1 text-[13px] font-bold text-[#FF6C37]"
            title="New request in folder"
            onclick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              addRequest(collectionId, item.id);
            }}>+</button
          >
          <button
            type="button"
            class="icon-btn hidden text-[10px] group-hover:inline-flex"
            onclick={() => startRename(item.id, item.name)}>✎</button
          >
          <button
            type="button"
            class="icon-btn hidden text-[10px] text-rose-400 group-hover:inline-flex"
            onclick={() => ondeleteItem(collectionId, item.id)}>×</button
          >
        {/if}
      </div>
      {#if isExpanded(item.id) && item.children}
        <ul class="ml-3 border-l border-app pl-1">
          {#each item.children as child, cidx (child.id)}
            {@render treeItem(collectionId, child, item.id, cidx, item.children.length)}
          {/each}
        </ul>
      {/if}
    {:else if item.request}
      {@const displayName = item.request.name || item.name}
      <div
        class="flex items-center gap-0.5 rounded-md
          {selectedId === item.id
          ? 'bg-[#FF6C37]/12 text-neutral-900 dark:text-white'
          : 'text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800/50'}"
      >
        {#if editingId === item.id}
          <span
            class="w-12 shrink-0 pl-2 font-mono text-[10px] font-bold {METHOD_COLORS[
              item.request.method
            ] ?? 'text-neutral-500 dark:text-neutral-400'}">{item.request.method}</span
          >
          <input
            class="input-field m-0.5 min-w-0 flex-1 py-1 text-xs"
            bind:value={editName}
            autofocus
            onfocus={(e) => e.currentTarget.select()}
            onkeydown={(e) => {
              if (e.key === "Enter") commitRename(collectionId, item.id);
              if (e.key === "Escape") editingId = null;
            }}
            onblur={() => commitRename(collectionId, item.id)}
          />
        {:else}
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-2 px-2 py-1.5 text-left text-sm transition"
            onclick={() => onselect(collectionId, item)}
            ondblclick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              startRename(item.id, displayName);
            }}
          >
            <span
              class="w-12 shrink-0 font-mono text-[10px] font-bold {METHOD_COLORS[
                item.request.method
              ] ?? 'text-neutral-500 dark:text-neutral-400'}">{item.request.method}</span
            >
            <span class="truncate" title="Double-click to rename">{displayName}</span>
          </button>
          <button
            type="button"
            class="icon-btn hidden text-[10px] group-hover:inline-flex"
            title="Rename"
            onclick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              startRename(item.id, displayName);
            }}>✎</button
          >
          <button
            type="button"
            class="icon-btn hidden text-[10px] text-rose-400 group-hover:inline-flex"
            title="Delete"
            onclick={() => ondeleteItem(collectionId, item.id)}>×</button
          >
          {#if index > 0}
            <button
              type="button"
              class="icon-btn hidden text-[10px] group-hover:inline-flex"
              title="Move up"
              onclick={() => onreorder(collectionId, parentId, item.id, index - 1)}
              >↑</button
            >
          {/if}
          {#if index < siblingCount - 1}
            <button
              type="button"
              class="icon-btn hidden text-[10px] group-hover:inline-flex"
              title="Move down"
              onclick={() => onreorder(collectionId, parentId, item.id, index + 1)}
              >↓</button
            >
          {/if}
        {/if}
      </div>
    {/if}
  </li>
{/snippet}
