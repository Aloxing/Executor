<script setup lang="ts">
import { FolderOpen, Loader2, RefreshCw, X } from "lucide-vue-next"
import { computed, ref } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import {
  reloadConfigProject,
  updateConfigProject,
  type ConfigProject,
  type ConfigQueue,
} from "@/lib/config"
import { showToast } from "@/lib/toast"
import { openInExplorer } from "@/lib/templates"

const props = defineProps<{
  project: ConfigProject
  /** Queue the project belongs to; needed by the update/reload commands. */
  queueUuid: string
}>()

const emit = defineEmits<{
  close: []
  saved: [queue: ConfigQueue]
}>()

// Imported projects are read-only here; disk projects can edit everything
// except the default info such as times.
const isImported = computed(() => props.project.source === "imported")

const name = ref(props.project.name)
const packageName = ref(props.project.packageName ?? "")
const error = ref("")
const saving = ref(false)
const reloading = ref(false)

// Closing and saving are driven by the central shortcut system; imported
// projects are read-only, so ctrl+s declines and falls through.
useShortcut("close", () => emit("close"))
useShortcut("save", () => {
  if (isImported.value) return false
  save()
  return true
})

async function save() {
  if (saving.value || isImported.value) return
  if (!name.value.trim()) {
    error.value = "请输入项目名称"
    return
  }
  if (!packageName.value.trim()) {
    error.value = "请输入项目包名"
    return
  }
  error.value = ""
  saving.value = true
  try {
    const queue = await updateConfigProject(
      props.queueUuid,
      props.project.uuid,
      name.value,
      packageName.value
    )
    showToast("项目信息已保存", "success")
    emit("saved", queue)
    emit("close")
  } catch (e) {
    error.value = typeof e === "string" ? e : "保存失败，请重试"
  } finally {
    saving.value = false
  }
}

// Clear the copied config directory and copy the import area's contents
// in again (imported projects only).
async function reload() {
  if (reloading.value || !isImported.value) return
  reloading.value = true
  error.value = ""
  try {
    const queue = await reloadConfigProject(props.queueUuid, props.project.uuid)
    showToast("已重新在导入区加载", "success")
    emit("saved", queue)
  } catch (e) {
    error.value = typeof e === "string" ? e : "重新加载失败，请重试"
  } finally {
    reloading.value = false
  }
}

async function locate() {
  try {
    await openInExplorer(props.project.rootPath)
  } catch (e) {
    showToast(typeof e === "string" ? e : "无法打开文件夹")
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
      aria-labelledby="config-project-info-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(90%,460px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="config-project-info-title"
          class="min-w-0 truncate text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          项目信息 · {{ project.name }}
        </h2>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div class="space-y-3 px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]">
        <!-- Row 1: project name -->
        <div class="space-y-1">
          <label
            for="info-project-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            项目名称
          </label>
          <input
            v-if="!isImported"
            id="info-project-name"
            v-model="name"
            type="text"
            placeholder="请输入项目名称"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
          <p
            v-else
            class="bg-muted/40 h-8 truncate rounded-lg border border-input px-3 leading-8 text-[clamp(11px,1.25vw,12px)]"
            :title="project.name"
          >
            {{ project.name }}
          </p>
        </div>
        <!-- Row 2: package name -->
        <div class="space-y-1">
          <label
            for="info-project-package"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            项目包名
          </label>
          <input
            v-if="!isImported"
            id="info-project-package"
            v-model="packageName"
            type="text"
            placeholder="请输入项目包名"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 font-mono text-[clamp(10px,1.1vw,11px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
          <p
            v-else
            class="bg-muted/40 h-8 truncate rounded-lg border border-input px-3 font-mono leading-8 text-[clamp(10px,1.1vw,11px)]"
            :title="project.packageName"
          >
            {{ project.packageName }}
          </p>
        </div>
        <!-- Config directory: locate icon on the right; imported projects
             additionally support reloading from the import area. -->
        <div class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            配置目录
          </span>
          <div class="flex items-center gap-2">
            <p
              class="bg-muted/40 text-muted-foreground h-8 min-w-0 flex-1 truncate rounded-lg border border-input px-3 font-mono leading-8 text-[clamp(10px,1.1vw,11px)]"
              :title="project.rootPath"
            >
              {{ project.rootPath }}
            </p>
            <button
              v-if="isImported"
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="reloading"
              aria-label="重新在导入区加载"
              title="清空配置目录并从导入区重新复制（仅导入项目）"
              @click="reload"
            >
              <Loader2 v-if="reloading" class="size-3.5 animate-spin" />
              <RefreshCw v-else class="size-3.5" />
            </button>
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
              aria-label="在资源管理器中打开配置目录"
              title="在资源管理器中打开配置目录"
              @click="locate"
            >
              <FolderOpen class="size-3.5" />
            </button>
          </div>
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
        class="flex shrink-0 items-center justify-between gap-2 border-t border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <!-- Config time as small text in the footer's left corner -->
        <p
          class="text-muted-foreground min-w-0 truncate text-[clamp(9px,1vw,10px)]"
          :title="`配置时间：${project.startedAt}`"
        >
          配置时间：{{ project.startedAt }}
        </p>
        <div class="flex shrink-0 items-center gap-2">
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="emit('close')"
          >
            {{ isImported ? "关闭" : "取消" }}
          </button>
          <button
            v-if="!isImported"
            type="button"
            class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:opacity-50"
            :disabled="saving"
            @click="save"
          >
            <Loader2 v-if="saving" class="size-3.5 animate-spin" />
            {{ saving ? "保存中…" : "保存" }}
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>
