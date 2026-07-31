<script lang="ts">
  import type { Environment, EnvironmentFile, KeyValue } from "$lib/types";
  import KeyValueEditor from "./KeyValueEditor.svelte";

  interface Props {
    envFile: EnvironmentFile;
    activeEnvId: string | null;
    onswitch: (id: string) => void;
    onchange: (file: EnvironmentFile) => void;
    ondelete: (id: string) => void;
  }

  let { envFile, activeEnvId, onswitch, onchange, ondelete }: Props = $props();

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
  {#if active}
    <div class="flex shrink-0 items-center gap-2 border-b border-app px-4 py-3">
      <div class="min-w-0 flex-1">
        <label class="mb-1 block text-[11px] text-neutral-500" for="env-name-input"
          >Environment name</label
        >
        <input
          id="env-name-input"
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
          <button type="button" class="chip" onclick={() => onswitch(active!.id)}
            >Set active</button
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
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
      <p class="text-sm">No environment selected</p>
      <p class="text-xs">Create one from the Env tab in the sidebar</p>
    </div>
  {/if}
</div>
