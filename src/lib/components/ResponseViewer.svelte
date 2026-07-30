<script lang="ts">
  import type {
    AssertionResult,
    BodyView,
    HttpResponse,
    ResponseTab,
  } from "$lib/types";
  import {
    formatBytes,
    formatDuration,
    statusBadgeBg,
    tryPrettyJson,
  } from "$lib/utils";

  interface Props {
    response: HttpResponse | null;
    error: string | null;
    sending: boolean;
    logs?: string[];
    assertions?: AssertionResult[];
  }

  let {
    response,
    error,
    sending,
    logs = [],
    assertions = [],
  }: Props = $props();

  let tab = $state<ResponseTab>("body");
  let view = $state<BodyView>("pretty");
  let search = $state("");

  let displayBody = $derived.by(() => {
    if (!response) return "";
    if (view === "pretty") {
      return tryPrettyJson(response.body) ?? response.body;
    }
    return response.body;
  });

  let filteredBody = $derived.by(() => {
    if (!search.trim()) return displayBody;
    const q = search.toLowerCase();
    return displayBody
      .split("\n")
      .filter((line) => line.toLowerCase().includes(q))
      .join("\n");
  });
</script>

<div class="flex h-full min-h-0 flex-col border-t border-app bg-neutral-50/80 dark:bg-neutral-950/40">
  <div class="flex items-center gap-3 border-b border-app px-3 py-2">
    <span class="text-xs font-semibold uppercase tracking-wider text-neutral-500"
      >Response</span
    >

    {#if response}
      <span
        class="rounded-full px-2 py-0.5 font-mono text-xs font-semibold ring-1 ring-inset {statusBadgeBg(
          response.status,
        )}"
      >
        {response.status}
        {response.statusText}
      </span>
      <span class="text-xs text-neutral-500 dark:text-neutral-400">{formatDuration(response.durationMs)}</span>
      <span class="text-xs text-neutral-500 dark:text-neutral-400">{formatBytes(response.bodySize)}</span>
    {:else if sending}
      <span class="text-xs text-[#FF6C37]">Waiting for response…</span>
    {:else if error}
      <span class="text-xs text-rose-400">Error</span>
    {:else}
      <span class="text-xs text-neutral-500">Send a request to see the response</span>
    {/if}

    <div class="ml-auto flex items-center gap-1">
      {#each [
        ["body", "Body"],
        ["headers", "Headers"],
        ["tests", "Tests"],
        ["console", "Console"],
      ] as [id, label]}
        <button
          type="button"
          class="tab-btn {tab === id ? 'tab-active' : ''}"
          onclick={() => (tab = id as ResponseTab)}>{label}</button
        >
      {/each}
    </div>
  </div>

  <div class="min-h-0 flex-1 overflow-auto p-3">
    {#if error}
      <div
        class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 font-mono text-sm text-rose-300"
      >
        {error}
      </div>
    {:else if sending}
      <div class="flex h-full items-center justify-center">
        <div class="flex flex-col items-center gap-3 text-neutral-500 dark:text-neutral-400">
          <div
            class="h-8 w-8 animate-spin rounded-full border-2 border-neutral-200 dark:border-neutral-700 border-t-[#FF6C37]"
          ></div>
          <span class="text-sm">Sending request…</span>
        </div>
      </div>
    {:else if tab === "console"}
      {#if logs.length === 0}
        <p class="text-sm text-neutral-500">No script logs.</p>
      {:else}
        <pre class="font-mono text-xs text-neutral-700 dark:text-neutral-300 whitespace-pre-wrap">{logs.join("\n")}</pre>
      {/if}
    {:else if tab === "tests"}
      {#if assertions.length === 0}
        <p class="text-sm text-neutral-500">No assertions yet. Use post-response scripts.</p>
      {:else}
        <ul class="space-y-1">
          {#each assertions as a}
            <li
              class="rounded-md border px-2 py-1.5 text-xs {a.passed
                ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300'
                : 'border-rose-500/30 bg-rose-500/10 text-rose-300'}"
            >
              <span class="font-semibold">{a.passed ? "PASS" : "FAIL"}</span>
              · {a.name}: {a.message}
            </li>
          {/each}
        </ul>
      {/if}
    {:else if !response}
      <div class="flex h-full items-center justify-center text-sm text-neutral-400 dark:text-neutral-600">
        Response will appear here
      </div>
    {:else if tab === "headers"}
      <table class="w-full text-left text-xs">
        <thead>
          <tr class="text-[11px] uppercase tracking-wide text-neutral-500">
            <th class="pb-2 pr-4 font-medium">Header</th>
            <th class="pb-2 font-medium">Value</th>
          </tr>
        </thead>
        <tbody>
          {#each response.headers as h}
            <tr class="border-t border-app">
              <td class="py-1.5 pr-4 font-mono text-neutral-700 dark:text-neutral-300">{h.key}</td>
              <td class="py-1.5 font-mono text-neutral-500 dark:text-neutral-400 break-all">{h.value}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="mb-2 flex items-center gap-2">
        <button
          type="button"
          class="chip {view === 'pretty' ? 'chip-active' : ''}"
          onclick={() => (view = "pretty")}>Pretty</button
        >
        <button
          type="button"
          class="chip {view === 'raw' ? 'chip-active' : ''}"
          onclick={() => (view = "raw")}>Raw</button
        >
        <input
          class="input-field ml-auto w-48 font-mono text-xs"
          placeholder="Search in body…"
          bind:value={search}
        />
      </div>
      <pre
        class="overflow-auto rounded-lg border border-app bg-neutral-50 dark:bg-neutral-900/50 p-3 font-mono text-xs leading-relaxed text-neutral-800 dark:text-neutral-200 whitespace-pre-wrap break-all"
        >{filteredBody || "(empty body)"}</pre
      >
    {/if}
  </div>
</div>
