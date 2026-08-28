<script setup lang="ts">
import { Loader2, X } from "lucide-vue-next"
import { onMounted, onUnmounted, ref } from "vue"
import { addConfigProject, type ConfigQueue } from "@/lib/config"

const props = defineProps<{
  queueUuid: string
  /** Directory picked from the file manager. */
  pickedPath: string
}>()

const emit = defineEmits<{
  close: []
  saved: [queue: ConfigQueue, name: string]
}>()

const name = ref("")
const packageName = ref("")
const error = ref("")
const saving = ref(false)

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown)
})

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})

// The entered name becomes the card name; the package name becomes a
// folder under non-local-package/ receiving the picked directory's
// contents, so the backend rejects duplicate package names.
async function submit() {
  if (saving.value) return
  if (!name.value.trim()) {
    error.value = "请输入配置名称"
    return
  }
  if (!packageName.value.trim()) {
    error.value = "请输入项目包名"
    return
  }
  error.value = ""
  saving.value = true
  try {
    const queue = await addConfigProject(props.queueUuid, {
      name: name.value,
      source: "disk",
      packageName: packageName.value,
      rootPath: props.pickedPath,
    })
    emit("saved", queue, name.value.trim())
    emit("close")
  } catch (e) {
    error.value = typeof e === "string" ? e : "创建失败，请重试"
  } finally {
    saving.value = false
  }
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
      aria-labelledby="create-disk-project-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(90%,460px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="create-disk-project-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          从磁盘中项目选择配置
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
      <div class="space-y-3 px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]">
        <div class="space-y-1">
          <label
            for="disk-project-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            配置名称
          </label>
          <input
            id="disk-project-name"
            v-model="name"
            type="text"
            placeholder="请输入配置名称"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <label
            for="disk-project-package"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            项目包名（不能重复，用作复制目录名称）
          </label>
          <input
            id="disk-project-package"
            v-model="packageName"
            type="text"
            placeholder="请输入项目包名"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 font-mono text-[clamp(10px,1.1vw,11px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            所选目录
          </span>
          <p
            class="bg-muted/40 text-muted-foreground flex h-8 items-center truncate rounded-lg border border-input px-3 font-mono text-[clamp(10px,1.1vw,11px)]"
            :title="pickedPath"
          >
            {{ pickedPath }}
          </p>
        </div>
        <p
          v-if="error"
          class="text-destructive text-[clamp(10px,1.1vw,11px)]"
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
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:opacity-50"
          :disabled="saving"
          @click="submit"
        >
          <Loader2 v-if="saving" class="size-3.5 animate-spin" />
          {{ saving ? "创建中…" : "创建" }}
        </button>
      </footer>
    </div>
  </div>
</template>
