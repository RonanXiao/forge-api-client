<script lang="ts">
  import type { Environment, EnvironmentFile, KeyValue } from "$lib/types";
  import { emptyKeyValue } from "$lib/utils";
  import KeyValueEditor from "./KeyValueEditor.svelte";

  interface Props {
    envFile: EnvironmentFile;
    activeEnvId: string | null;
    onswitch: (id: string) => void;
    onchange: (file: EnvironmentFile) => void;
    onadd: () => void;
    ondelete: (id: string) => void;
    onclose: () => void;
  }

  let {
    envFile,
    activeEnvId,
    onswitch,
    onchange,
    onadd,
    ondelete,
    onclose,
  }: Props = $props();

  let active = $derived(
    envFile.environments.find((e) => e.id === activeEnvId) ??
      envFile.environments[0] ??
      null,
  );

  function patchEnv(id: string, partial: Partial<Environment>) {
    onchange({
      ...envFile,
      environments: envFile.environments.map((e) =>
        e.id === id ? { ...e, ...partial } : e,
      ),
    });
  }

  function setVariables(variables: KeyValue[]) {
    if (!active) return;
    patchEnv(active.id, { variables });
  }

  function setName(name: string) {
    if (!active) return;
    patchEnv(active.id, { name });
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-white dark:bg-neutral-950">
  <div
    class="flex h-11 shrink-0 items-center gap-3 border-b border-app px-4"
  >
    <h1 class="text-sm font-semibold tracking-tight">Environments</h1>
    <span class="text-[11px] text-neutral-500">
      Manage variables used as <code class="font-mono text-[#FF6C37]">{"{{name}}"}</code> in
      requests
    </span>
    <button type="button" class="chip ml-auto" onclick={onclose}>← Back to request</button>
  </div>

  <div class="flex min-h-0 flex-1">
    <!-- Left: environment list -->
    <aside
      class="flex w-56 shrink-0 flex-col border-r border-app bg-neutral-50 dark:bg-neutral-950"
    >
      <div class="flex items-center justify-between border-b border-app px-3 py-2">
        <span
          class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500"
          >Environments</span
        >
        <button type="button" class="chip px-2 py-0.5 text-[11px]" onclick={onadd}
          >+ New</button
        >
      </div>
      <ul class="min-h-0 flex-1 overflow-y-auto p-2">
        {#each envFile.environments as env (env.id)}
          <li class="mb-0.5">
            <div
              class="group flex items-center gap-0.5 rounded-md
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
                onclick={() => onswitch(env.id)}
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
                class="mr-1 hidden h-7 w-7 shrink-0 items-center justify-center rounded text-neutral-400 group-hover:flex hover:bg-rose-50 hover:text-rose-500 dark:hover:bg-rose-950/40"
                title="Delete environment"
                onclick={(e) => {
                  e.stopPropagation();
                  ondelete(env.id);
                }}>×</button
              >
            </div>
          </li>
        {:else}
          <li class="px-2 py-8 text-center text-xs text-neutral-500">
            No environments yet
          </li>
        {/each}
      </ul>
    </aside>

    <!-- Right: variables -->
    <section class="flex min-w-0 flex-1 flex-col">
      {#if active}
        <div
          class="flex shrink-0 items-center gap-2 border-b border-app px-4 py-3"
        >
          <div class="min-w-0 flex-1">
            <label class="mb-1 block text-[11px] text-neutral-500">Environment name</label>
            <input
              class="input-field w-full max-w-md text-sm font-medium"
              value={active.name}
              oninput={(e) => setName(e.currentTarget.value)}
            />
          </div>
          <div class="flex shrink-0 items-end gap-2 self-end pb-0.5">
            {#if active.id === activeEnvId}
              <span
                class="rounded-full bg-[#FF6C37]/15 px-2 py-0.5 text-[11px] font-medium text-[#FF6C37]"
                >Active</span
              >
            {:else}
              <button
                type="button"
                class="chip"
                onclick={() => onswitch(active!.id)}>Set active</button
              >
            {/if}
            <button
              type="button"
              class="chip text-rose-500 hover:bg-rose-50 dark:hover:bg-rose-950/40"
              onclick={() => ondelete(active!.id)}>Delete</button
            >
          </div>
        </div>
        <div class="min-h-0 flex-1 overflow-auto p-4">
          <p class="mb-3 text-[11px] text-neutral-500">
            Variables are available as <code class="font-mono">{"{{variable}}"}</code> in URL,
            headers, body, and scripts.
          </p>
          <KeyValueEditor
            items={active.variables}
            keyPlaceholder="Variable"
            valuePlaceholder="Value"
            onchange={setVariables}
          />
        </div>
      {:else}
        <div
          class="flex flex-1 flex-col items-center justify-center gap-3 text-neutral-500"
        >
          <p class="text-sm">No environment selected</p>
          <button type="button" class="btn-primary" onclick={onadd}>Create environment</button>
        </div>
      {/if}
    </section>
  </div>
</div>
