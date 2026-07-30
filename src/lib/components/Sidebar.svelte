<script lang="ts">
  import type { Collection, CollectionItem, HistoryEntry } from "$lib/types";
  import { METHOD_COLORS } from "$lib/utils";

  interface Props {
    collections: Collection[];
    history: HistoryEntry[];
    selectedId: string | null;
    workspacePath: string;
    onselect: (collectionId: string, item: CollectionItem) => void;
    onnewRequest: (collectionId: string, parentId?: string | null) => void;
    onnewFolder: (collectionId: string) => void;
    onnewCollection: () => void;
    ondeleteCollection: (id: string) => void;
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
  }

  let {
    collections,
    history,
    selectedId,
    workspacePath,
    onselect,
    onnewRequest,
    onnewFolder,
    onnewCollection,
    ondeleteCollection,
    onrename,
    ondeleteItem,
    onreorder,
    onselectHistory,
    onclearHistory,
  }: Props = $props();

  let panel = $state<"collections" | "history">("collections");
  let expanded = $state<Record<string, boolean>>({});
  let editingId = $state<string | null>(null);
  let editName = $state("");
  let dragId = $state<string | null>(null);

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
  <div class="flex items-center gap-1 border-b border-app p-2">
    <button
      type="button"
      class="tab-btn flex-1 {panel === 'collections' ? 'tab-active' : ''}"
      onclick={() => (panel = "collections")}
    >
      Collections
    </button>
    <button
      type="button"
      class="tab-btn flex-1 {panel === 'history' ? 'tab-active' : ''}"
      onclick={() => (panel = "history")}
    >
      History
    </button>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if panel === "collections"}
      <div class="mb-2 flex items-center justify-between px-1">
        <span class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500"
          >Local</span
        >
        <button type="button" class="icon-btn text-xs" onclick={onnewCollection}
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
              class="icon-btn inline-flex shrink-0 text-[13px] font-semibold text-[#FF6C37]"
              title="New request"
              onclick={(e) => {
                e.stopPropagation();
                onnewRequest(col.id);
              }}>+</button
            >
            <button
              type="button"
              class="icon-btn inline-flex shrink-0 text-[11px]"
              title="New folder"
              onclick={(e) => {
                e.stopPropagation();
                onnewFolder(col.id);
              }}>📁</button
            >
            <button
              type="button"
              class="icon-btn inline-flex shrink-0 text-[11px] text-rose-500"
              title="Delete collection"
              onclick={(e) => {
                e.stopPropagation();
                ondeleteCollection(col.id);
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
    {:else}
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
            class="icon-btn inline-flex shrink-0 text-[12px] font-semibold text-[#FF6C37]"
            title="New request in folder"
            onclick={(e) => {
              e.stopPropagation();
              onnewRequest(collectionId, item.id);
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
      <div class="flex items-center">
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition
            {selectedId === item.id
            ? 'bg-[#FF6C37]/12 text-neutral-900 dark:text-white'
            : 'text-neutral-700 dark:text-neutral-300 hover:bg-neutral-100 dark:bg-neutral-800/50'}"
          onclick={() => onselect(collectionId, item)}
          ondblclick={() => startRename(item.id, item.name)}
        >
          <span
            class="w-12 shrink-0 font-mono text-[10px] font-bold {METHOD_COLORS[
              item.request.method
            ] ?? 'text-neutral-500 dark:text-neutral-400'}">{item.request.method}</span
          >
          {#if editingId === item.id}
            <input
              class="input-field flex-1 py-0.5 text-xs"
              bind:value={editName}
              onclick={(e) => e.stopPropagation()}
              onkeydown={(e) => e.key === "Enter" && commitRename(collectionId, item.id)}
              onblur={() => commitRename(collectionId, item.id)}
            />
          {:else}
            <span class="truncate">{item.name}</span>
          {/if}
        </button>
        <button
          type="button"
          class="icon-btn hidden text-[10px] text-rose-400 group-hover:inline-flex"
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
      </div>
    {/if}
  </li>
{/snippet}
