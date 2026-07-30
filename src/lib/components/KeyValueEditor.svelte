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
    items = [],
    keyPlaceholder = "Key",
    valuePlaceholder = "Value",
    onchange,
  }: Props = $props();

  /**
   * Postman-style: always show a trailing blank row to type into.
   * Parent may omit empty keys (e.g. form body storage) — we re-add the row for UI.
   */
  let displayRows = $derived.by((): KeyValue[] => {
    const list = (items ?? []).map((r) => ({
      key: r.key ?? "",
      value: r.value ?? "",
      enabled: r.enabled !== false,
    }));
    const last = list[list.length - 1];
    if (!last || last.key.trim() !== "" || last.value.trim() !== "") {
      return [...list, emptyKeyValue()];
    }
    return list;
  });

  function emit(next: KeyValue[]) {
    // Drop pure trailing blanks — UI re-adds one empty row
    let cleaned = [...next];
    while (cleaned.length > 0) {
      const last = cleaned[cleaned.length - 1];
      if (!last.key.trim() && !last.value.trim()) {
        cleaned.pop();
      } else {
        break;
      }
    }
    onchange?.(cleaned);
  }

  function isBlank(row: KeyValue): boolean {
    return !row.key.trim() && !row.value.trim();
  }

  function update(i: number, field: keyof KeyValue, value: string | boolean) {
    const list = displayRows.map((row, idx) => {
      if (idx !== i) return { ...row };
      const next = { ...row, [field]: value };
      // First time user types into an empty row → enable (show check)
      if (
        field !== "enabled" &&
        isBlank(row) &&
        !isBlank(next)
      ) {
        next.enabled = true;
      }
      return next;
    });
    emit(list);
  }

  function remove(i: number) {
    // Never leave UI without the trailing blank (displayRows handles that)
    emit(displayRows.filter((_, idx) => idx !== i));
  }
</script>

<div class="overflow-hidden rounded-md border border-app">
  <div
    class="grid grid-cols-[36px_minmax(0,1fr)_minmax(0,1fr)_36px] border-b border-app bg-neutral-50 text-[11px] font-medium text-neutral-500 dark:bg-neutral-900/60"
  >
    <span class="flex items-center justify-center py-2"></span>
    <span class="border-l border-app px-2 py-2">{keyPlaceholder}</span>
    <span class="border-l border-app px-2 py-2">{valuePlaceholder}</span>
    <span></span>
  </div>

  {#each displayRows as row, i (i)}
    {@const blank = isBlank(row)}
    <div
      class="grid grid-cols-[36px_minmax(0,1fr)_minmax(0,1fr)_36px] items-stretch border-b border-app last:border-b-0"
    >
      <div class="flex items-center justify-center">
        {#if !blank}
          <label class="flex items-center justify-center">
            <input
              type="checkbox"
              class="h-3.5 w-3.5 rounded border-neutral-300 bg-white text-[#FF6C37] focus:ring-[#FF6C37]/40 dark:border-neutral-600 dark:bg-neutral-800"
              checked={row.enabled}
              onchange={(e) => update(i, "enabled", e.currentTarget.checked)}
            />
          </label>
        {/if}
      </div>
      <input
        class="min-w-0 border-0 border-l border-app bg-transparent px-2 py-2 font-mono text-xs outline-none placeholder:text-neutral-400 focus:bg-[#FF6C37]/5 dark:placeholder:text-neutral-600"
        placeholder={keyPlaceholder}
        value={row.key}
        oninput={(e) => update(i, "key", e.currentTarget.value)}
      />
      <input
        class="min-w-0 border-0 border-l border-app bg-transparent px-2 py-2 font-mono text-xs outline-none placeholder:text-neutral-400 focus:bg-[#FF6C37]/5 dark:placeholder:text-neutral-600"
        placeholder={valuePlaceholder}
        value={row.value}
        oninput={(e) => update(i, "value", e.currentTarget.value)}
      />
      <button
        type="button"
        class="flex items-center justify-center border-l border-app text-neutral-400 hover:bg-neutral-50 hover:text-rose-500 dark:hover:bg-neutral-800 {blank
          ? 'invisible'
          : ''}"
        title="Remove"
        tabindex={blank ? -1 : 0}
        onclick={() => remove(i)}
      >
        ×
      </button>
    </div>
  {/each}
</div>
