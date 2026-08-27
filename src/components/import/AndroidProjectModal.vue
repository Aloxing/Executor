<script setup lang="ts">
import { FolderOpen, Loader2, MapPin, RefreshCw, X } from "lucide-vue-next"
import { computed, onMounted, onUnmounted, ref } from "vue"
import {
  addAndroidProject,
  getAndroidProjectDir,
  reloadAndroidProject,
  updateAndroidProject,
  type AndroidProject,
} from "@/lib/android"
import { formatNow, openInExplorer } from "@/lib/templates"

const props = defineProps<{
  queueUuid: string
  /** When provided the modal switches to edit mode for this project. */
  initial?: AndroidProject
}>()

const emit = defineEmits<{
  close: []
  saved: [project: AndroidProject]
  reloaded: [project: AndroidProject]
}>()

const editMode = computed(() => !!props.initial)

const appName = ref(props.initial?.appName ?? "")
const packageName = ref(props.initial?.packageName ?? "")
const rootPath = ref(props.initial?.rootPath ?? "")
const createdAt = props.initial?.createdAt ?? formatNow()
const updatedAt = formatNow()
// Imported project location (edit mode only); empty when not imported yet.
const projectDir = ref("")
const error = ref("")
const saving = ref(false)
const reloading = ref(false)

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown)
  if (props.initial) {
    try {
      projectDir.value = await getAndroidProjectDir(props.initial.packageName)
    } catch {
      projectDir.value = ""
    }
  }
})

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})

async function pickRootPath() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择下载路径",
    })
    if (selected) rootPath.value = selected as string
  } catch (e) {
    error.value = typeof e === "string" ? e : "打开目录选择器失败"
  }
}

/** Clears the imported contents and re-imports from the download path. */
async function reloadFromSource() {
  if (!props.initial || reloading.value || saving.value) return
  reloading.value = true
  error.value = ""
  try {
    const project = await reloadAndroidProject(props.initial.packageName)
    emit("reloaded", project)
  } catch (e) {
    error.value = typeof e === "string" ? e : "重新加载失败，请重试"
  } finally {
    reloading.value = false
  }
}

async function locateProject() {
  if (!projectDir.value) return
  try {
    await openInExplorer(projectDir.value)
  } catch (e) {
    error.value = typeof e === "string" ? e : "打开资源管理器失败"
  }
}

async function submit() {
  if (saving.value) return
  if (!appName.value.trim()) {
    error.value = "请输入应用名称"
    return
  }
  if (!packageName.value.trim()) {
    error.value = "请输入应用包名"
    return
  }
  if (!rootPath.value.trim()) {
    error.value = "请选择下载路径"
    return
  }
  error.value = ""
  saving.value = true
  try {
    const data = {
      appName: appName.value,
      packageName: packageName.value,
      rootPath: rootPath.value,
    }
    const project = editMode.value
      ? await updateAndroidProject(props.initial!.packageName, data)
      : await addAndroidProject(props.queueUuid, data)
    emit("saved", project)
    emit("close")
  } catch (e) {
    error.value = typeof e === "string" ? e : "保存失败，请重试"
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
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="android-project-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(90%,460px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="android-project-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          {{ editMode ? "修改 Android 项目" : "添加 Android 项目" }}
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
            for="app-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            应用名称
          </label>
          <input
            id="app-name"
            v-model="appName"
            type="text"
            placeholder="请输入应用名称"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <label
            for="package-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            应用包名
          </label>
          <input
            id="package-name"
            v-model="packageName"
            type="text"
            placeholder="如 com.example.app"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <label
            for="root-path"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            下载路径
          </label>
          <div class="flex items-center gap-2">
            <input
              id="root-path"
              v-model="rootPath"
              type="text"
              placeholder="请选择或输入下载路径"
              class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
            />
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
              aria-label="选择下载路径"
              title="选择目录"
              @click="pickRootPath"
            >
              <FolderOpen class="size-3.5" />
            </button>
          </div>
        </div>
        <!-- Imported project location (edit mode only) -->
        <div v-if="editMode" class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            项目位置（导入后）
          </span>
          <div class="flex items-center gap-2">
            <p
              class="bg-muted/40 text-muted-foreground h-8 min-w-0 flex-1 truncate rounded-lg border border-input px-3 leading-8 text-[clamp(10px,1.1vw,11px)]"
              :title="projectDir || '尚未导入'"
            >
              {{ projectDir || "尚未导入，导入后位于工作空间 import/package/ 下" }}
            </p>
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="!projectDir"
              aria-label="在资源管理器中定位项目位置"
              title="在资源管理器中定位"
              @click="locateProject"
            >
              <MapPin class="size-3.5" />
            </button>
          </div>
        </div>
        <!-- Re-import from the download path (edit mode only) -->
        <button
          v-if="editMode"
          type="button"
          class="hover:bg-muted inline-flex h-8 w-full cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-muted/60 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="reloading || saving"
          @click="reloadFromSource"
        >
          <Loader2 v-if="reloading" class="size-3.5 animate-spin" />
          <RefreshCw v-else class="size-3.5" />
          {{ reloading ? "重新加载中…" : "从下载路径重新加载项目" }}
        </button>
        <div
          class="text-muted-foreground flex items-center justify-between text-[clamp(10px,1.1vw,11px)]"
        >
          <span>创建时间：{{ createdAt }}</span>
          <span>修改时间：{{ updatedAt }}</span>
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
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:opacity-50"
          :disabled="saving"
          @click="submit"
        >
          {{ editMode ? "保存" : "添加" }}
        </button>
      </footer>
    </div>
  </div>
</template>
