<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState } from "@codemirror/state";
  import { json } from "@codemirror/lang-json";
  import { javascript } from "@codemirror/lang-javascript";
  import { oneDark } from "@codemirror/theme-one-dark";

  interface Props {
    value: string;
    language?: "json" | "javascript" | "text";
    dark?: boolean;
    minHeight?: string;
    onchange?: (value: string) => void;
  }

  let {
    value = $bindable(""),
    language = "text",
    dark = false,
    minHeight = "180px",
    onchange,
  }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let skip = false;

  function langExt() {
    if (language === "json") return json();
    if (language === "javascript") return javascript();
    return [];
  }

  onMount(() => {
    if (!host) return;
    const extensions = [
      basicSetup,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) {
          skip = true;
          const next = u.state.doc.toString();
          value = next;
          onchange?.(next);
          skip = false;
        }
      }),
      EditorView.theme({
        "&": { minHeight, fontSize: "12px" },
        ".cm-scroller": { overflow: "auto", minHeight },
      }),
      langExt(),
    ];
    if (dark) extensions.push(oneDark);

    view = new EditorView({
      state: EditorState.create({ doc: value, extensions }),
      parent: host,
    });
  });

  $effect(() => {
    if (!view || skip) return;
    const cur = view.state.doc.toString();
    if (value !== cur) {
      view.dispatch({
        changes: { from: 0, to: cur.length, insert: value },
      });
    }
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });
</script>

<div class="overflow-hidden rounded-md border border-neutral-200 dark:border-neutral-700" bind:this={host}></div>
