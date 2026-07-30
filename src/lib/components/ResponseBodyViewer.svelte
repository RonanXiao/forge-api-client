<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, lineNumbers, highlightActiveLineGutter } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import {
    foldGutter,
    foldKeymap,
    syntaxHighlighting,
    defaultHighlightStyle,
    foldAll,
    unfoldAll,
    indentOnInput,
    bracketMatching,
  } from "@codemirror/language";
  import { json } from "@codemirror/lang-json";
  import { xml } from "@codemirror/lang-xml";
  import { oneDark } from "@codemirror/theme-one-dark";
  import { keymap, highlightSpecialChars, drawSelection } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { tryPrettyJson } from "$lib/utils";

  interface Props {
    body: string;
    pretty?: boolean;
    search?: string;
    dark?: boolean;
  }

  let {
    body = "",
    pretty = true,
    search = "",
    dark = false,
  }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let langLabel = $state<"JSON" | "XML" | "Text">("Text");
  const langCompartment = new Compartment();
  const themeCompartment = new Compartment();

  function detectLang(text: string): "json" | "xml" | "text" {
    const t = text.trim();
    if (!t) return "text";
    if (t.startsWith("{") || t.startsWith("[")) {
      try {
        JSON.parse(t);
        return "json";
      } catch {
        /* fallthrough */
      }
    }
    if (t.startsWith("<") && t.includes(">")) return "xml";
    return "text";
  }

  /** Minimal XML pretty-print for fold-friendly structure */
  function prettyXml(raw: string): string {
    const t = raw.trim();
    try {
      const P = new DOMParser();
      const doc = P.parseFromString(t, "application/xml");
      if (doc.querySelector("parsererror")) return raw;
    } catch {
      return raw;
    }
    // indent by tags
    let formatted = "";
    let indent = 0;
    const parts = t.replace(/>\s*</g, ">\n<").split("\n");
    for (const line of parts) {
      const l = line.trim();
      if (!l) continue;
      if (l.startsWith("</")) indent = Math.max(0, indent - 1);
      formatted += "  ".repeat(indent) + l + "\n";
      if (
        l.startsWith("<") &&
        !l.startsWith("</") &&
        !l.startsWith("<?") &&
        !l.startsWith("<!") &&
        !l.endsWith("/>") &&
        !l.includes("</")
      ) {
        indent += 1;
      }
    }
    return formatted.trimEnd() + "\n";
  }

  function prepareDoc(raw: string, doPretty: boolean): { text: string; lang: "json" | "xml" | "text" } {
    const lang = detectLang(raw);
    if (!doPretty) return { text: raw, lang };
    if (lang === "json") {
      return { text: tryPrettyJson(raw) ?? raw, lang };
    }
    if (lang === "xml") {
      return { text: prettyXml(raw), lang };
    }
    return { text: raw, lang };
  }

  function langExt(lang: "json" | "xml" | "text") {
    if (lang === "json") return json();
    if (lang === "xml") return xml();
    return [];
  }

  function applySearchHighlight(text: string, q: string): string {
    // Keep full text for folding; search just scrolls/selects first match
    return text;
  }

  function buildExtensions(lang: "json" | "xml" | "text", isDark: boolean) {
    const exts = [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      drawSelection(),
      indentOnInput(),
      bracketMatching(),
      foldGutter({
        openText: "▼",
        closedText: "▶",
      }),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap]),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      EditorView.theme({
        "&": {
          fontSize: "12px",
          height: "100%",
          maxHeight: "100%",
        },
        ".cm-scroller": {
          overflow: "auto",
          fontFamily:
            'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
        },
        ".cm-content": {
          minHeight: "120px",
        },
        ".cm-gutters": {
          borderRight: isDark ? "1px solid #404040" : "1px solid #e5e5e5",
          backgroundColor: isDark ? "#171717" : "#fafafa",
        },
        ".cm-foldGutter .cm-gutterElement": {
          cursor: "pointer",
          color: isDark ? "#a3a3a3" : "#737373",
          padding: "0 4px",
        },
      }),
      langCompartment.of(langExt(lang)),
      themeCompartment.of(isDark ? oneDark : []),
    ];
    return exts;
  }

  function mountOrUpdate() {
    if (!host) return;
    const prepared = prepareDoc(body ?? "", pretty);
    langLabel =
      prepared.lang === "json" ? "JSON" : prepared.lang === "xml" ? "XML" : "Text";
    const text = applySearchHighlight(prepared.text, search);

    if (!view) {
      view = new EditorView({
        state: EditorState.create({
          doc: text,
          extensions: buildExtensions(prepared.lang, dark),
        }),
        parent: host,
      });
    } else {
      view.setState(
        EditorState.create({
          doc: text,
          extensions: buildExtensions(prepared.lang, dark),
        }),
      );
    }

    // Jump to first search match
    if (search.trim() && view) {
      const q = search.toLowerCase();
      const full = view.state.doc.toString().toLowerCase();
      const idx = full.indexOf(q);
      if (idx >= 0) {
        view.dispatch({
          selection: { anchor: idx, head: idx + search.length },
          scrollIntoView: true,
        });
      }
    }
  }

  export function collapseAll() {
    if (view) foldAll(view);
  }

  export function expandAll() {
    if (view) unfoldAll(view);
  }

  onMount(() => {
    mountOrUpdate();
  });

  $effect(() => {
    // track deps
    void body;
    void pretty;
    void search;
    void dark;
    if (host) mountOrUpdate();
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="mb-1 flex items-center gap-2 text-[11px] text-neutral-500">
    <span
      class="rounded px-1.5 py-0.5 font-semibold
        {langLabel === 'JSON'
        ? 'bg-amber-500/15 text-amber-700 dark:text-amber-400'
        : langLabel === 'XML'
          ? 'bg-sky-500/15 text-sky-700 dark:text-sky-400'
          : 'bg-neutral-200/80 text-neutral-600 dark:bg-neutral-800 dark:text-neutral-400'}"
      >{langLabel}</span
    >
    {#if langLabel === "JSON" || langLabel === "XML"}
      <span class="text-neutral-400">Click gutters ▶/▼ to fold · buttons to fold all</span>
    {/if}
  </div>
  <div
    class="min-h-0 flex-1 overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700"
    bind:this={host}
  ></div>
</div>
