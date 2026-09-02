<script setup lang="ts">
import { ListPlus, Loader2, Search, X } from "lucide-vue-next"
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue"
import AppSelect from "../AppSelect.vue"
import ConfirmDialog from "../import/ConfirmDialog.vue"
import BuildProjectContextMenu from "./BuildProjectContextMenu.vue"
import BuildQueueCard from "./BuildQueueCard.vue"
import BuildQueueContextMenu from "./BuildQueueContextMenu.vue"
import CreateBuildQueueModal from "./CreateBuildQueueModal.vue"
import PickConfiguredProjectModal from "./PickConfiguredProjectModal.vue"
import {
  addBuildProject,
  listBuildQueues,
  listenBuildLog,
  removeBuildProject,
  runProjectBuild,
  stopProjectBuild,
  type BuildLogEvent,
  type BuildProject,
  type BuildQueue,
  type ConfiguredPick,
} from "@/lib/build"
import { settings } from "@/lib/settings"
import { useShortcut } from "@/lib/shortcuts"
import { openInExplorer } from "@/lib/templates"
import { showToast } from "@/lib/toast"

const queues = ref<BuildQueue[]>([])

// Build type selector; only Android is supported for now.
const buildType = ref("android")
const buildTypeOptions = [{ value: "android", label: "Android" }]

// Gradle environment selector (configured in 设置 → 编译, persisted with
// the settings in the data directory).
const gradleEnv = ref("")
const gradleEnvOptions = computed(() =>
  settings.gradleEnvs.map((env) => ({ value: env.path, label: env.name }))
)
watch(
  gradleEnvOptions,
  (options) => {
    if (!options.some((o) => o.value === gradleEnv.value)) {
      gradleEnv.value = options[0]?.value ?? ""
    }
  },
  { immediate: true }
)

// Create-queue modal state.
const showCreate = ref(false)

// Page-level shortcut: primary create button.
useShortcut("create", () => {
  showCreate.value = true
})
// Right-click context menu states.
const menu = ref<{ x: number; y: number; queue: BuildQueue } | null>(null)
const projectMenu = ref<{
  x: number
  y: number
  queue: BuildQueue
  project: BuildProject
} | null>(null)
// Queue currently receiving projects through the pick modal.
const pickQueue = ref<BuildQueue | null>(null)
// Project waiting for remove confirmation.
const pendingRemove = ref<{ queue: BuildQueue; project: BuildProject } | null>(null)
const removing = ref(false)

// Project/queue currently building (drives the loading indicators).
const buildingUuid = ref("")
const queueBuildingUuid = ref("")
// Set when the user stops a build-all run so the loop breaks early.
const stopRequested = ref(false)

// --- Build logs ---------------------------------------------------------------

interface LogEntry {
  name: string
  lines: string[]
  status: "" | "running" | "success" | "failed"
}

// One log page per built project; the tab bar switches between them.
const logs = ref<Record<string, LogEntry>>({})
const logOrder = ref<string[]>([])
const activeLog = ref("")
const logBodyRef = ref<HTMLElement | null>(null)
// Prefix/keyword filter applied to the visible log lines.
const logFilter = ref("")

/** Per-page line cap: older lines are released so a long build never
 * grows the DOM/memory without bound. */
const MAX_LOG_LINES = 100

function ensureLog(uuid: string, name: string) {
  if (!logs.value[uuid]) {
    logs.value[uuid] = { name, lines: [], status: "" }
    logOrder.value.push(uuid)
  }
  logs.value[uuid].status = "running"
  activeLog.value = uuid
}

// Removes a log page (tab); a running build must be stopped first.
function removeLog(uuid: string) {
  const entry = logs.value[uuid]
  if (entry?.status === "running") {
    showToast("构建进行中，请先停止构建再移除日志")
    return
  }
  delete logs.value[uuid]
  logOrder.value = logOrder.value.filter((id) => id !== uuid)
  if (activeLog.value === uuid) {
    activeLog.value = logOrder.value[logOrder.value.length - 1] ?? ""
  }
}

async function scrollLogToBottom() {
  await nextTick()
  const el = logBodyRef.value
  if (el) el.scrollTop = el.scrollHeight
}

let unlistenLog: (() => void) | undefined

function onBuildLog(event: BuildLogEvent) {
  const entry = logs.value[event.projectUuid]
  if (!entry) return
  if (event.kind === "done") {
    entry.status = event.success ? "success" : "failed"
  } else {
    // The backend coalesces output into multi-line chunks.
    entry.lines.push(...event.line.split("\n"))
    if (entry.lines.length > MAX_LOG_LINES) {
      entry.lines.splice(0, entry.lines.length - MAX_LOG_LINES)
    }
  }
  scrollLogToBottom()
}

// Visible lines of the active log after the prefix/keyword filter.
const displayLines = computed(() => {
  if (!activeLog.value) return [] as string[]
  const entry = logs.value[activeLog.value]
  if (!entry) return [] as string[]
  const kw = logFilter.value.trim().toLowerCase()
  return kw
    ? entry.lines.filter((line) => line.toLowerCase().includes(kw))
    : entry.lines
})

async function reload() {
  queues.value = await listBuildQueues()
}

onMounted(async () => {
  await reload()
  try {
    unlistenLog = await listenBuildLog(onBuildLog)
  } catch {
    // Not running inside Tauri.
  }
})

onUnmounted(() => {
  unlistenLog?.()
  unlistenLog = undefined
})

function replaceQueue(updated: BuildQueue) {
  queues.value = queues.value.map((q) =>
    q.uuid === updated.uuid ? updated : q
  )
}

function onQueueSaved(queue: BuildQueue) {
  queues.value.push(queue)
  showToast(`构建队列「${queue.name}」创建成功`, "success")
}

function onContextMenu(queue: BuildQueue, event: MouseEvent) {
  menu.value = { x: event.clientX, y: event.clientY, queue }
}

function onProjectContextMenu(
  queue: BuildQueue,
  project: BuildProject,
  event: MouseEvent
) {
  projectMenu.value = { x: event.clientX, y: event.clientY, queue, project }
}

// --- Adding projects -----------------------------------------------------------

function openPickConfig() {
  if (!menu.value) return
  pickQueue.value = menu.value.queue
  menu.value = null
}

// Config-area projects are recorded by their existing address; nothing is
// copied into the build area.
async function onPickConfirm(picks: ConfiguredPick[]) {
  const queue = pickQueue.value
  if (!queue || !picks.length) return
  pickQueue.value = null
  let added = 0
  let error = ""
  for (const pick of picks) {
    try {
      const updated = await addBuildProject(queue.uuid, {
        name: pick.name,
        source: "config",
        packageName: pick.packageName || undefined,
        rootPath: pick.rootPath,
      })
      replaceQueue(updated)
      added++
    } catch (e) {
      error = typeof e === "string" ? e : "添加失败，请重试"
    }
  }
  if (added) {
    showToast(`已为队列「${queue.name}」添加 ${added} 个项目`, "success")
  }
  if (error) showToast(error)
}

async function pickFromDisk() {
  if (!menu.value) return
  const queue = menu.value.queue
  menu.value = null
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择项目目录",
    })
    if (!selected) return
    const rootPath = selected as string
    const updated = await addBuildProject(queue.uuid, {
      name: dirName(rootPath),
      source: "disk",
      rootPath,
    })
    replaceQueue(updated)
    showToast(`项目「${dirName(rootPath)}」已添加到队列「${queue.name}」`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "添加失败，请重试")
  }
}

/** Last path segment of a Windows/POSIX path, trailing separators ignored. */
function dirName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "")
  return trimmed.split(/[\\/]/).pop() ?? trimmed
}

// --- Project removal ------------------------------------------------------------

function onDeleteProject(queue: BuildQueue, project: BuildProject) {
  pendingRemove.value = { queue, project }
}

async function confirmRemove() {
  const target = pendingRemove.value
  if (!target || removing.value) return
  removing.value = true
  try {
    const updated = await removeBuildProject(
      target.queue.uuid,
      target.project.uuid
    )
    replaceQueue(updated)
    pendingRemove.value = null
    // The project is gone, so its log page goes with it.
    removeLog(target.project.uuid)
    showToast("已从队列删除卡片", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    removing.value = false
  }
}

// --- Building --------------------------------------------------------------------

function requireGradleEnv(): string | null {
  if (!gradleEnv.value) {
    showToast("请先在「设置 → 编译」添加 Gradle 环境，并在工具栏选择")
    return null
  }
  return gradleEnv.value
}

// Single project build: gradle wrapper (selected env) → gradlew <args>.
async function onBuildProject(args: string[]) {
  const target = projectMenu.value
  if (!target) return
  projectMenu.value = null
  const env = requireGradleEnv()
  if (!env || buildingUuid.value || queueBuildingUuid.value) return
  ensureLog(target.project.uuid, target.project.name)
  buildingUuid.value = target.project.uuid
  try {
    await runProjectBuild(target.project.uuid, env, args)
  } catch (e) {
    showToast(typeof e === "string" ? e : "构建失败，请查看日志")
  } finally {
    buildingUuid.value = ""
  }
}

// Locate the recorded project directory in the explorer.
async function onLocateProject() {
  const target = projectMenu.value
  if (!target) return
  projectMenu.value = null
  if (!target.project.rootPath) {
    showToast("该项目暂无可定位的目录")
    return
  }
  try {
    await openInExplorer(target.project.rootPath)
  } catch (e) {
    showToast(typeof e === "string" ? e : "定位失败")
  }
}

// Build-all: the same flow for every project of the queue, one after
// another; each project gets its own log page.
async function onBuildAll(args: string[]) {
  if (!menu.value) return
  const queue = menu.value.queue
  menu.value = null
  if (!queue.projects.length) {
    showToast(`队列「${queue.name}」下暂无项目`)
    return
  }
  const env = requireGradleEnv()
  if (!env || buildingUuid.value || queueBuildingUuid.value) return
  queueBuildingUuid.value = queue.uuid
  stopRequested.value = false
  for (const project of queue.projects) {
    if (stopRequested.value) {
      showToast("已停止队列构建", "info")
      break
    }
    ensureLog(project.uuid, project.name)
    buildingUuid.value = project.uuid
    try {
      await runProjectBuild(project.uuid, env, args)
    } catch (e) {
      showToast(typeof e === "string" ? e : `「${project.name}」构建失败，请查看日志`)
    } finally {
      buildingUuid.value = ""
    }
  }
  queueBuildingUuid.value = ""
  stopRequested.value = false
  showToast(`队列「${queue.name}」构建流程执行完毕`, "success")
}

// Stop the running build of one project (kills the whole process tree);
// during a build-all run it also breaks the loop before the next project.
async function onStopProject(project: BuildProject) {
  const entry = logs.value[project.uuid]
  if (entry) {
    entry.lines.push("== 已请求停止构建 ==")
    scrollLogToBottom()
  }
  if (queueBuildingUuid.value) stopRequested.value = true
  try {
    await stopProjectBuild(project.uuid)
  } catch (e) {
    showToast(typeof e === "string" ? e : "停止构建失败")
  }
}
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      构建区
    </h1>
    <div class="flex shrink-0 items-center gap-3">
      <!-- Left cluster: build type + gradle environment selectors -->
      <div class="flex items-center gap-2">
        <span class="text-muted-foreground shrink-0 text-[clamp(11px,1.25vw,12px)]">
          构建类型
        </span>
        <div class="w-[clamp(100px,11vw,140px)]">
          <AppSelect
            v-model="buildType"
            :options="buildTypeOptions"
            aria-label="构建类型"
          />
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-muted-foreground shrink-0 text-[clamp(11px,1.25vw,12px)]">
          Gradle 环境
        </span>
        <div v-if="gradleEnvOptions.length" class="w-[clamp(140px,15vw,200px)]">
          <AppSelect
            v-model="gradleEnv"
            :options="gradleEnvOptions"
            aria-label="Gradle 环境"
          />
        </div>
        <span
          v-else
          class="text-muted-foreground text-[clamp(10px,1.1vw,11px)]"
        >
          未配置，请先在「设置 → 编译」添加
        </span>
      </div>
      <!-- Right cluster: primary action -->
      <div class="ml-auto flex shrink-0 items-center gap-3">
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="showCreate = true"
        >
          <ListPlus class="size-3.5" />
          创建构建队列
        </button>
      </div>
    </div>
    <div class="flex min-h-0 flex-1 gap-3">
      <!-- Queue directory: one quarter of the page width with its own
           scrollbar, mirroring the import/config pages. -->
      <aside
        class="flex min-h-0 w-1/4 shrink-0 flex-col rounded-xl border border-border"
      >
        <header
          class="flex shrink-0 items-center justify-between px-3 py-2"
        >
          <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">队列目录</h2>
          <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
            共 {{ queues.length }} 个
          </span>
        </header>
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
          <template v-if="queues.length">
            <BuildQueueCard
              v-for="queue in queues"
              :key="queue.uuid"
              :queue="queue"
              :building-uuid="buildingUuid"
              :building="queueBuildingUuid === queue.uuid"
              @delete-project="onDeleteProject(queue, $event)"
              @stop-project="onStopProject"
              @contextmenu="onContextMenu(queue, $event)"
              @project-contextmenu="(project, event) => onProjectContextMenu(queue, project, event)"
            />
          </template>
          <div v-else class="flex flex-1 items-center justify-center">
            <p class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]">
              暂无队列，点击右上角创建
            </p>
          </div>
        </div>
      </aside>
      <!-- Build log area: one tab per built project -->
      <aside
        class="flex min-h-0 min-w-0 flex-1 flex-col rounded-xl border border-border"
      >
        <header
          class="flex shrink-0 items-center justify-between px-3 py-2"
        >
          <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">构建日志</h2>
          <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
            共 {{ logOrder.length }} 个
          </span>
        </header>
        <!-- Log tabs: pick which project's log to view; the X removes a
             log page (blocked while its build is running) -->
        <div
          v-if="logOrder.length"
          class="flex shrink-0 gap-1 overflow-x-auto border-b border-border px-2 pb-1.5"
        >
          <div
            v-for="uuid in logOrder"
            :key="uuid"
            role="tab"
            class="inline-flex h-6 shrink-0 cursor-pointer items-center gap-1.5 rounded-md px-2 text-[clamp(10px,1.1vw,11px)] font-medium transition-colors duration-200"
            :class="
              activeLog === uuid
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-accent/60 hover:text-accent-foreground bg-transparent'
            "
            :title="logs[uuid]?.name"
            :aria-selected="activeLog === uuid"
            @click="activeLog = uuid"
          >
            <Loader2
              v-if="logs[uuid]?.status === 'running'"
              class="size-2.5 shrink-0 animate-spin"
            />
            <span
              v-else-if="logs[uuid]?.status === 'success'"
              class="size-1.5 shrink-0 rounded-full bg-emerald-500"
              aria-hidden="true"
            />
            <span
              v-else-if="logs[uuid]?.status === 'failed'"
              class="size-1.5 shrink-0 rounded-full bg-red-500"
              aria-hidden="true"
            />
            <span class="max-w-[140px] truncate">{{ logs[uuid]?.name }}</span>
            <button
              type="button"
              class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 inline-flex size-4 shrink-0 cursor-pointer items-center justify-center rounded transition-colors duration-200 focus-visible:outline-none"
              aria-label="移除日志"
              title="移除该日志页"
              @click.stop="removeLog(uuid)"
            >
              <X class="size-2.5" />
            </button>
          </div>
        </div>
        <!-- Log filter -->
        <div
          v-if="logOrder.length"
          class="relative flex shrink-0 items-center border-b border-border px-2 py-1.5"
        >
          <Search
            class="text-muted-foreground pointer-events-none absolute left-4 top-1/2 size-3 -translate-y-1/2"
          />
          <input
            v-model="logFilter"
            type="text"
            placeholder="筛选日志（包含匹配，如 Task / FAILURE）"
            class="bg-background focus-visible:ring-ring h-6 w-full rounded-md border border-input pr-6 pl-6 text-[clamp(9px,1vw,10px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
          <button
            v-if="logFilter"
            type="button"
            class="text-muted-foreground hover:text-foreground absolute right-3.5 top-1/2 inline-flex size-4 -translate-y-1/2 cursor-pointer items-center justify-center rounded bg-transparent transition-colors focus-visible:outline-none"
            aria-label="清除日志筛选"
            @click="logFilter = ''"
          >
            <X class="size-2.5" />
          </button>
        </div>
        <div ref="logBodyRef" class="min-h-0 flex-1 overflow-auto p-2">
          <pre
            v-if="activeLog && logs[activeLog]"
            class="text-muted-foreground font-mono text-[clamp(9px,1vw,10px]) leading-relaxed whitespace-pre-wrap break-all"
            >{{ displayLines.join("\n") }}</pre
          >
          <div v-else class="flex h-full items-center justify-center">
            <p class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]">
              暂无构建日志，右键队列或项目卡片开始构建
            </p>
          </div>
        </div>
      </aside>
    </div>
    <CreateBuildQueueModal
      v-if="showCreate"
      @close="showCreate = false"
      @saved="onQueueSaved"
    />
    <BuildQueueContextMenu
      v-if="menu"
      :x="menu.x"
      :y="menu.y"
      :queue="menu.queue"
      @close="menu = null"
      @pick-config="openPickConfig"
      @pick-disk="pickFromDisk"
      @build-all="onBuildAll"
    />
    <BuildProjectContextMenu
      v-if="projectMenu"
      :x="projectMenu.x"
      :y="projectMenu.y"
      :project="projectMenu.project"
      @close="projectMenu = null"
      @build="onBuildProject"
      @locate="onLocateProject"
    />
    <PickConfiguredProjectModal
      v-if="pickQueue"
      @close="pickQueue = null"
      @confirm="onPickConfirm"
    />
    <ConfirmDialog
      v-if="pendingRemove"
      title="从队列删除卡片"
      :message="`确定将项目「${pendingRemove.project.name}」从队列「${pendingRemove.queue.name}」中删除吗？项目文件不会被删除。`"
      :busy="removing"
      @cancel="pendingRemove = null"
      @confirm="confirmRemove"
    />
  </div>
</template>
