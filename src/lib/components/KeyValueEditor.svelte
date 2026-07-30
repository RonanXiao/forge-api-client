<script lang="ts">
  import type { KeyValue } from "$lib/types";
  import { emptyKeyValue } from "$lib/utils";

  interface Props {
    items: KeyValue[];
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    onchange?: (items: KeyValue[]) => void;
  }

  let {
    items = $bindable(),
    keyPlaceholder = "Key",
    valuePlaceholder = "Value",
    onchange,
  }: Props = $props();

  function emit(next: KeyValue[]) {
    items = next;
    onchange?.(next);
  }

  function update(i: number, field: keyof KeyValue, value: string | boolean) {
    const next = items.map((row, idx) =>
      idx === i ? { ...row, [field]: value } : row,
    );
    const last = next[next.length - 1];
    if (last && (last.key || last.value)) {
      next.push(emptyKeyValue());
    }
    emit(next);
  }

  function remove(i: number) {
    if (items.length <= 1) {
      emit([emptyKeyValue()]);
      return;
    }
    emit(items.filter((_, idx) => idx !== i));
  }
</script>

<div class="flex flex-col gap-1">
  <div
    class="grid grid-cols-[28px_1fr_1fr_28px] gap-1 px-1 text-[11px] font-medium uppercase tracking-wide text-slate-500"
  >
    <span></span>
    <span>{keyPlaceholder}</span>
    <span>{valuePlaceholder}</span>
    <span></span>
  </div>
  {#each items as row, i (i)}
    <div class="grid grid-cols-[28px_1fr_1fr_28px] items-center gap-1">
      <label class="flex items-center justify-center">
        <input
          type="checkbox"
          class="h-3.5 w-3.5 rounded border-slate-600 bg-slate-800 text-indigo-500 focus:ring-indigo-500/40"
          checked={row.enabled}
          onchange={(e) => update(i, "enabled", e.currentTarget.checked)}
        />
      </label>
      <input
        class="input-field font-mono text-xs"
        placeholder={keyPlaceholder}
        value={row.key}
        oninput={(e) => update(i, "key", e.currentTarget.value)}
      />
      <input
        class="input-field font-mono text-xs"
        placeholder={valuePlaceholder}
        value={row.value}
        oninput={(e) => update(i, "value", e.currentTarget.value)}
      />
      <button
        type="button"
        class="flex h-8 w-7 items-center justify-center rounded text-slate-500 hover:bg-slate-800 hover:text-rose-400"
        title="Remove"
        onclick={() => remove(i)}
      >
        ×
      </button>
    </div>
  {/each}
</div>
