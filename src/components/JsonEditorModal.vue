<script setup lang="ts">
import { EditorState } from "@codemirror/state"
import { EditorView } from "@codemirror/view"
import { json } from "@codemirror/lang-json"
import { oneDark } from "@codemirror/theme-one-dark"
import { basicSetup } from "codemirror"
import { X } from "lucide-vue-next"
import { onMounted, onUnmounted, ref } from "vue"
import { useShortcut } from "@/lib/shortcuts"

const props = defineProps<{
  title: string
  /** Initial JSON content to edit. */
  modelValue: string
}>()

const emit = defineEmits<{
  close: []
  save: [content: string]
}>()

const content = ref(props.modelValue)
const error = ref("")
const containerRef = ref<HTMLDivElement | null>(null)
let editor: EditorView | null = null

// Closing and saving are driven by the central shortcut system.
useShortcut("close", () => emit("close"))
useShortcut("save", save)

onMounted(() => {
  createEditor()
})

onUnmounted(() => {
  editor?.destroy()
  editor = null
})

// CodeMirror 6 with the official JSON language package: real editor
// cursor/selection handling, no overlay alignment issues.
function createEditor() {
  if (!containerRef.value) return
  const dark = document.documentElement.classList.contains("dark")
  editor = new EditorView({
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        basicSetup,
        json(),
        EditorView.lineWrapping,
        ...(dark ? [oneDark] : []),
        EditorView.theme({
          "&": { height: "100%", fontSize: "12px", backgroundColor: "transparent" },
          ".cm-scroller": {
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
            lineHeight: "1.6",
          },
          ".cm-gutters": { border: "none" },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            content.value = update.state.doc.toString()
          }
        }),
      ],
    }),
    parent: containerRef.value,
  })
}

function save() {
  try {
    JSON.parse(content.value)
  } catch (e) {
    error.value = `JSON 格式错误：${e instanceof Error ? e.message : String(e)}`
    return
  }
  error.value = ""
  emit("save", content.value)
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-[3%]">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0 backdrop-blur-sm"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      class="animate-modal-enter bg-card text-card-foreground relative flex h-[min(86%,680px)] w-[min(92%,860px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2 class="text-[clamp(12px,1.5vw,13px)] font-semibold">
          {{ title }}
        </h2>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div
        class="flex min-h-0 flex-1 flex-col gap-2 px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]"
      >
        <!-- CodeMirror editor mounts here. -->
        <div
          ref="containerRef"
          class="bg-background min-h-0 flex-1 overflow-hidden rounded-lg border border-input"
        />
        <p
          v-if="error"
          class="text-destructive shrink-0 break-words text-[clamp(10px,1.1vw,11px)]"
          role="alert"
        >
          {{ error }}
        </p>
      </div>
      <footer
        class="flex shrink-0 items-center justify-end gap-2 border-t border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('close')"
        >
          取消
        </button>
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="save"
        >
          保存
        </button>
      </footer>
    </div>
  </div>
</template>
