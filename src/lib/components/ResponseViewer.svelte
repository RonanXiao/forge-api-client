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
  } from "$lib/utils";
  import ResponseBodyViewer from "./ResponseBodyViewer.svelte";

  interface Props {
    response: HttpResponse | null;
    error: string | null;
    sending: boolean;
    logs?: string[];
    assertions?: AssertionResult[];
    dark?: boolean;
  }

  let {
    response,
    error,
    sending,
    logs = [],
    assertions = [],
    dark = false,
  }: Props = $props();

  let tab = $state<ResponseTab>("body");
  let view = $state<BodyView>("pretty");
  let search = $state("");
  let bodyViewer = $state<ReturnType<typeof ResponseBodyViewer> | null>(null);

  function collapseAll() {
    bodyViewer?.collapseAll();
  }

  function expandAll() {
    bodyViewer?.expandAll();
  }

  /** Short network error from response (timeout etc.) — not the global script error */
  let sendError = $derived(response?.error?.trim() || null);
  let verboseText = $derived(
    (response?.verbose && response.verbose.trim()) || null,
  );

  // Prefer short error for Body; never dump verbose trace here
  let shortError = $derived.by(() => {
    if (sendError) return sendError;
    if (!error) return null;
    if (error.includes("--- verbose ---")) {
      return error.split("\n\n--- verbose ---")[0]?.trim() || "Request failed";
    }
    return error;
  });
</script>

<div class="flex h-full min-h-0 flex-col border-t border-app bg-neutral-50/80 dark:bg-neutral-950/40">
  <div class="flex items-center gap-3 border-b border-app px-3 py-2">
    <span class="text-xs font-semibold uppercase tracking-wider text-neutral-500"
      >Response</span
    >

    {#if response && !sendError}
      <span
        class="rounded-full px-2 py-0.5 font-mono text-xs font-semibold ring-1 ring-inset {statusBadgeBg(
          response.status,
        )}"
      >
        {response.status}
        {response.statusText}
      </span>
      <span class="text-xs text-neutral-500 dark:text-neutral-400"
        >{formatDuration(response.durationMs)}</span
      >
      <span class="text-xs text-neutral-500 dark:text-neutral-400"
        >{formatBytes(response.bodySize)}</span
      >
    {:else if sendError || (response && response.status === 0)}
      <span
        class="rounded-full bg-rose-500/15 px-2 py-0.5 font-mono text-xs font-semibold text-rose-600 ring-1 ring-inset ring-rose-500/30 dark:text-rose-400"
        >Error</span
      >
      {#if response}
        <span class="text-xs text-neutral-500 dark:text-neutral-400"
          >{formatDuration(response.durationMs)}</span
        >
      {/if}
    {:else if sending}
      <span class="text-xs text-[#FF6C37]">Waiting for response…</span>
    {:else if error}
      <span class="text-xs text-rose-400">Error</span>
    {:else}
      <span class="text-xs text-neutral-500">Send a request to see the response</span>
    {/if}

    <div class="ml-auto flex items-center gap-1">
      {#each [
        ["verbose", "Verbose"],
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

  <div class="flex min-h-0 flex-1 flex-col overflow-hidden p-3">
    {#if sending}
      <div class="flex h-full items-center justify-center">
        <div class="flex flex-col items-center gap-3 text-neutral-500 dark:text-neutral-400">
          <div
            class="h-8 w-8 animate-spin rounded-full border-2 border-neutral-200 dark:border-neutral-700 border-t-[#FF6C37]"
          ></div>
          <span class="text-sm">Sending request…</span>
        </div>
      </div>
    {:else if tab === "verbose"}
      {#if verboseText}
        <div class="mb-2 flex items-center gap-2">
          <span class="text-[11px] text-neutral-500"
            >curl -v style trace (connection · request · response)</span
          >
          <button
            type="button"
            class="chip ml-auto"
            onclick={() => {
              void navigator.clipboard.writeText(verboseText ?? "");
            }}>Copy</button
          >
        </div>
        <pre
          class="min-h-0 flex-1 overflow-auto rounded-md border border-app bg-white p-3 font-mono text-[12px] leading-relaxed whitespace-pre-wrap dark:bg-neutral-900"
          >{#each verboseText.split("\n") as line}{#if line.startsWith("*")}<span
                class="text-neutral-400 dark:text-neutral-500">{line}</span
              >{"\n"}{:else if line.startsWith(">")}<span
                class="text-emerald-600 dark:text-emerald-400">{line}</span
              >{"\n"}{:else if line.startsWith("<")}<span
                class="text-amber-600 dark:text-amber-300">{line}</span
              >{"\n"}{:else}<span class="text-neutral-700 dark:text-neutral-300"
                >{line}</span
              >{"\n"}{/if}{/each}</pre
        >
      {:else if shortError}
        <pre
          class="min-h-0 flex-1 overflow-auto rounded-md border border-app bg-white p-3 font-mono text-[12px] leading-relaxed text-rose-600 whitespace-pre-wrap dark:bg-neutral-900 dark:text-rose-300"
          >{shortError}</pre
        >
      {:else}
        <p class="text-sm text-neutral-500">
          Send a request to see verbose debug output (like curl -v).
        </p>
      {/if}
    {:else if tab === "console"}
      {#if logs.length === 0}
        <p class="text-sm text-neutral-500">No script logs.</p>
      {:else}
        <pre
          class="font-mono text-xs text-neutral-700 dark:text-neutral-300 whitespace-pre-wrap"
          >{logs.join("\n")}</pre
        >
      {/if}
    {:else if tab === "tests"}
      {#if assertions.length === 0}
        <p class="text-sm text-neutral-500">No assertions yet. Use post-response scripts.</p>
      {:else}
        <ul class="space-y-1">
          {#each assertions as a}
            <li
              class="rounded-md border px-2 py-1.5 text-xs {a.passed
                ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
                : 'border-rose-500/30 bg-rose-500/10 text-rose-700 dark:text-rose-300'}"
            >
              <span class="font-semibold">{a.passed ? "PASS" : "FAIL"}</span>
              · {a.name}: {a.message}
            </li>
          {/each}
        </ul>
      {/if}
    {:else if !response && !shortError}
      <div
        class="flex h-full items-center justify-center text-sm text-neutral-400 dark:text-neutral-600"
      >
        Response will appear here
      </div>
    {:else if tab === "headers"}
      {#if response && response.headers.length > 0}
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
                <td class="py-1.5 pr-4 font-mono text-neutral-700 dark:text-neutral-300"
                  >{h.key}</td
                >
                <td
                  class="py-1.5 font-mono text-neutral-500 dark:text-neutral-400 break-all"
                  >{h.value}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="text-sm text-neutral-500">No response headers.</p>
      {/if}
    {:else if tab === "body"}
      {#if shortError && (!response?.body || response.body.length === 0)}
        <!-- Body: short error only; open Verbose for full curl -v dump -->
        <div
          class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-3 text-sm text-rose-700 dark:text-rose-300"
        >
          <p class="font-medium">{shortError}</p>
          {#if verboseText}
            <p class="mt-2 text-[11px] text-rose-600/80 dark:text-rose-400/80">
              See the <button
                type="button"
                class="underline decoration-rose-400 underline-offset-2"
                onclick={() => (tab = "verbose")}>Verbose</button
              > tab for full request trace.
            </p>
          {/if}
        </div>
      {:else if response}
      <div class="mb-2 flex flex-wrap items-center gap-2">
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
        {#if view === "pretty"}
          <button type="button" class="chip" title="Collapse all" onclick={collapseAll}
            >Collapse all</button
          >
          <button type="button" class="chip" title="Expand all" onclick={expandAll}
            >Expand all</button
          >
        {/if}
        <input
          class="input-field ml-auto w-48 font-mono text-xs"
          placeholder="Search in body…"
          bind:value={search}
        />
      </div>
      <div class="min-h-0 flex-1">
        <ResponseBodyViewer
          bind:this={bodyViewer}
          body={response.body}
          pretty={view === "pretty"}
          {search}
          {dark}
        />
      </div>
      {:else if shortError}
        <div
          class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-3 text-sm text-rose-700 dark:text-rose-300"
        >
          <p class="font-medium">{shortError}</p>
        </div>
      {/if}
    {/if}
  </div>
</div>
