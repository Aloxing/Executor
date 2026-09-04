<script setup lang="ts">
import { closeBrackets } from "@codemirror/autocomplete"
import { indentWithTab } from "@codemirror/commands"
import { java } from "@codemirror/lang-java"
import { bracketMatching, indentOnInput } from "@codemirror/language"
import { Compartment, EditorState } from "@codemirror/state"
import { oneDark } from "@codemirror/theme-one-dark"
import {
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers,
} from "@codemirror/view"
import { minimalSetup } from "codemirror"
import { onMounted, onUnmounted, ref, watch } from "vue"

/**
 * Inline Java-method editor of the parameter card's 代码模式.
 *
 * CodeMirror 6 on the minimal setup (history, draw-selection, the default
 * keymap and the fallback highlight style) plus the Java language, line
 * numbers, bracket handling and Tab indenting — deliberately none of
 * `basicSetup`'s panels, whose search keymap would fight the app's own
 * Ctrl+F shortcut.
 *
 * The cursor is styled explicitly: `drawSelection` paints its own caret
 * (the native one is transparent), so its colour and width come from the
 * theme variables here instead of the library default, which is a hairline
 * black that all but disappears on the dark surface. The pointer is set
 * explicitly too — the surrounding project card is `cursor-pointer` and
 * `select-none`, and both inherit into the editor.
 */
const props = defineProps<{
  /** Method source generated from the entry's `scenes`. */
  modelValue: string
}>()

const emit = defineEmits<{
  "update:modelValue": [value: string]
}>()

const containerRef = ref<HTMLDivElement | null>(null)
let editor: EditorView | null = null

// oneDark sits in a compartment so switching the theme recolours the Java
// highlighting live: the app only toggles the `dark` class on <html>
// (lib/theme.ts), it does not notify anyone.
const darkTheme = new Compartment()
let themeObserver: MutationObserver | null = null

function isDark(): boolean {
  return document.documentElement.classList.contains("dark")
}

onMounted(() => {
  createEditor()
  themeObserver = new MutationObserver(() => {
    editor?.dispatch({
      effects: darkTheme.reconfigure(isDark() ? [oneDark] : []),
    })
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  })
})

onUnmounted(() => {
  themeObserver?.disconnect()
  themeObserver = null
  editor?.destroy()
  editor = null
})

// Regenerated content (re-entering the mode, a card refresh) replaces the
// document; the guard keeps the editor's own change events from looping
// and the cursor from jumping while the user types.
watch(
  () => props.modelValue,
  (value) => {
    if (!editor) return
    const current = editor.state.doc.toString()
    if (current === value) return
    editor.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    })
  }
)

function createEditor() {
  if (!containerRef.value) return
  editor = new EditorView({
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        minimalSetup,
        // Java tokens drive the highlighting (minimalSetup already carries
        // the fallback style; oneDark supplies its own palette when dark).
        java(),
        lineNumbers(),
        highlightActiveLine(),
        bracketMatching(),
        closeBrackets(),
        indentOnInput(),
        // Tab / Shift-Tab indent and outdent the selection.
        keymap.of([indentWithTab]),
        EditorView.lineWrapping,
        // Dark surfaces get oneDark's palette; light ones fall back to the
        // default highlight style that minimalSetup already registers.
        darkTheme.of(isDark() ? [oneDark] : []),
        EditorView.theme({
          "&": {
            height: "100%",
            fontSize: "clamp(10px,1.05vw,11px)",
            backgroundColor: "transparent",
          },
          "&.cm-focused": { outline: "none" },
          ".cm-scroller": {
            fontFamily:
              "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
            lineHeight: "1.65",
          },
          ".cm-gutters": {
            border: "none",
            backgroundColor: "transparent",
            // The gutter is not editable: arrow, not a text caret.
            cursor: "default",
          },
          ".cm-content": {
            padding: "6px 0",
            // Inherited from the card's cursor-pointer / select-none.
            cursor: "text",
            userSelect: "text",
          },
          ".cm-line": { paddingLeft: "2px" },
          // Caret: painted by drawSelection, so it needs an explicit colour
          // and a width that survives the theme switch (var(--foreground)
          // follows the light/dark surface automatically).
          ".cm-cursor, .cm-dropCursor": {
            borderLeftColor: "var(--foreground)",
            borderLeftWidth: "2px",
          },
          ".cm-activeLine": {
            backgroundColor: "color-mix(in oklab, var(--muted) 55%, transparent)",
          },
          ".cm-activeLineGutter": { backgroundColor: "transparent" },
          ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
            backgroundColor: "color-mix(in oklab, var(--primary) 25%, transparent)",
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            emit("update:modelValue", update.state.doc.toString())
          }
        }),
      ],
    }),
    parent: containerRef.value,
  })
}
</script>

<template>
  <div
    ref="containerRef"
    class="scene-code-host bg-background h-full min-h-0 overflow-hidden rounded-md border border-input"
  />
</template>

<style scoped>
/* CodeMirror sizes itself to the host; the scroller owns the overflow so
 * the gutter stays put while the code scrolls. */
.scene-code-host :deep(.cm-editor) {
  height: 100%;
}

.scene-code-host :deep(.cm-scroller) {
  overflow: auto;
}
</style>
