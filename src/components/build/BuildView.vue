<script setup lang="ts">
import { FolderOpen, ListPlus, Loader2, Search, Smartphone, X } from "lucide-vue-next"
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref, watch } from "vue"
import AppSelect from "../AppSelect.vue"
import ConfirmDialog from "../import/ConfirmDialog.vue"
import BuildModeModal from "./BuildModeModal.vue"
import BuildProjectContextMenu from "./BuildProjectContextMenu.vue"
import BuildQueueCard from "./BuildQueueCard.vue"
import BuildQueueContextMenu from "./BuildQueueContextMenu.vue"
import CreateBuildQueueModal from "./CreateBuildQueueModal.vue"
import DeviceLogModal from "./DeviceLogModal.vue"
import PickConfiguredProjectModal from "./PickConfiguredProjectModal.vue"
import {
  addBuildProject,
  clearBuildQueue,
  deleteBuildQueues,
  getBuildLogsDir,
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
import {
  deviceLogId,
  listenDeviceLog,
  listAndroidDevices,
  startDeviceLogcat,
  stopDeviceLogcat,
  type AndroidDevice,
  type DeviceLogEvent,
} from "@/lib/devices"
import { notifySystem } from "@/lib/notify"
import { byCreatedAt } from "@/lib/queues"
import { settings } from "@/lib/settings"
import { useShortcut } from "@/lib/shortcuts"
import { pendingBuildRequest } from "@/lib/pipeline"
import { openInExplorer } from "@/lib/templates"
import { showToast } from "@/lib/toast"

const queues = ref<BuildQueue[]>([])

// Queue cards display newest-first (降序，最新的排第一).
const sortedQueues = computed(() => [...queues.value].sort(byCreatedAt))

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

// Projects currently building (parallel builds track several at once);
// drives the per-card spinners and stop buttons.
const buildingUuids = ref<Set<string>>(new Set())
// Queue whose build-all is running.
const queueBuildingUuid = ref("")
// Queue waiting in the build-mode dialog (command + serial/parallel).
const buildAllQueue = ref<BuildQueue | null>(null)
// Queue waiting for its delete confirmation.
const pendingDeleteQueue = ref<BuildQueue | null>(null)
const deletingQueue = ref(false)
// Queue waiting for its clear confirmation.
const pendingClearQueue = ref<BuildQueue | null>(null)
const clearingQueue = ref(false)
// Set when the user stops a build-all run so the loop breaks early.
const stopRequested = ref(false)

// --- Build logs ---------------------------------------------------------------

interface LogEntry {
  name: string
  lines: string[]
  /** Lines matching the active filter; own cap, never flushed by the
   * tail-only main buffer. */
  filtered: string[]
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
// App-scoped device logs are much quieter than builds, so their pages
// keep a deeper history.
const DEVICE_MAX_LINES = 500
// Filter matches accumulate in their own buffer (per log page), so a
// long build can never flush them out of the tail-only main buffer.
const FILTERED_MAX_LINES = 500

function ensureLog(uuid: string, name: string) {
  if (!logs.value[uuid]) {
    logs.value[uuid] = { name, lines: [], filtered: [], status: "" }
    logOrder.value.push(uuid)
  }
  logs.value[uuid].status = "running"
  activeLog.value = uuid
}

/** Appends the filter-matching lines to the entry's own buffer. */
function collectFiltered(entry: LogEntry, lines: string[]) {
  const kw = logFilter.value.trim().toLowerCase()
  if (!kw) return
  for (const line of lines) {
    if (line.toLowerCase().includes(kw)) {
      entry.filtered.push(line)
    }
  }
  if (entry.filtered.length > FILTERED_MAX_LINES) {
    entry.filtered.splice(0, entry.filtered.length - FILTERED_MAX_LINES)
  }
}

// Changing the filter re-seeds every filtered buffer from the retained
// tails; new matching lines keep accumulating from then on.
watch(logFilter, () => {
  const kw = logFilter.value.trim().toLowerCase()
  for (const entry of Object.values(logs.value)) {
    entry.filtered = kw
      ? entry.lines.filter((line) => line.toLowerCase().includes(kw))
      : []
  }
})

// Removes a log page (tab); a running build must be stopped first.
// Device pages stop their logcat capture and go straight away.
function removeLog(uuid: string) {
  if (uuid.startsWith("device:")) {
    stopDeviceLogcat(uuid.slice("device:".length)).catch(() => {})
  } else {
    const entry = logs.value[uuid]
    if (entry?.status === "running") {
      showToast("构建进行中，请先停止构建再移除日志")
      return
    }
  }
  delete logs.value[uuid]
  logOrder.value = logOrder.value.filter((id) => id !== uuid)
  if (activeLog.value === uuid) {
    activeLog.value = logOrder.value[logOrder.value.length - 1] ?? ""
  }
}

async function scrollLogToBottom() {
  // While the user has scrolled away from the bottom, new lines must not
  // yank the view back; auto-follow resumes at the bottom edge.
  if (!stickToBottom.value) return
  await nextTick()
  const el = logBodyRef.value
  if (el) el.scrollTop = el.scrollHeight
}

// True while the log body sits (near) its bottom edge.
const stickToBottom = ref(true)

function onLogScroll() {
  const el = logBodyRef.value
  if (!el) return
  stickToBottom.value = el.scrollHeight - el.scrollTop - el.clientHeight < 40
}

// Switching log pages always starts following the tail again.
watch(activeLog, () => {
  stickToBottom.value = true
  scrollLogToBottom()
})

let unlistenLog: (() => void) | undefined
let unlistenDeviceLog: (() => void) | undefined
// True while the USB-device scan is running.
const scanningDevices = ref(false)

/** Splits one streamed chunk into lines; adb/cmd on Windows emit CRLF,
 * and a stray trailing \r breaks both rendering and level detection. */
function splitChunk(chunk: string): string[] {
  return chunk.split("\n").map((line) =>
    line.endsWith("\r") ? line.slice(0, -1) : line
  )
}

function onBuildLog(event: BuildLogEvent) {
  const entry = logs.value[event.projectUuid]
  if (!entry) return
  if (event.kind === "done") {
    entry.status = event.success ? "success" : "failed"
  } else {
    // The backend coalesces output into multi-line chunks.
    const lines = splitChunk(event.line)
    entry.lines.push(...lines)
    if (entry.lines.length > MAX_LOG_LINES) {
      entry.lines.splice(0, entry.lines.length - MAX_LOG_LINES)
    }
    collectFiltered(entry, lines)
  }
  scrollLogToBottom()
}

// Device logcat chunks land in the device's own log page.
function onDeviceLog(event: DeviceLogEvent) {
  const entry = logs.value[deviceLogId(event.serial)]
  if (!entry) return
  if (event.kind === "done") {
    entry.status = event.success ? "success" : "failed"
  } else {
    const lines = splitChunk(event.line)
    entry.lines.push(...lines)
    if (entry.lines.length > DEVICE_MAX_LINES) {
      entry.lines.splice(0, entry.lines.length - DEVICE_MAX_LINES)
    }
    collectFiltered(entry, lines)
  }
  scrollLogToBottom()
}

// 设备日志: detect USB-debug devices first, then the modal picks which
// devices to capture and whether to filter by an app package.
const deviceModal = ref<AndroidDevice[] | null>(null)

// Opens the persistent log-cache directory (build/logs): full histories
// of every build and device capture, kept across restarts.
async function onOpenLogsDir() {
  try {
    const dir = await getBuildLogsDir()
    await openInExplorer(dir)
  } catch (e) {
    showToast(typeof e === "string" ? e : "打开日志缓存目录失败")
  }
}

async function onCaptureDevices() {
  if (scanningDevices.value) return
  scanningDevices.value = true
  try {
    const devices = await listAndroidDevices()
    const online = devices.filter((d) => d.status === "device")
    if (!online.length) {
      showToast(
        devices.length
          ? "检测到设备但未授权或离线，请在手机上确认 USB 调试授权"
          : "未检测到已开启 USB 调试的 Android 设备"
      )
      return
    }
    deviceModal.value = online
  } catch (e) {
    showToast(typeof e === "string" ? e : "检测设备失败")
  } finally {
    scanningDevices.value = false
  }
}

// Starts one capture page per selected device; with a package name the
// backend attaches to the app process (waiting/re-attaching as needed).
function onStartDeviceCapture(serials: string[], packageName: string) {
  const devices = deviceModal.value ?? []
  deviceModal.value = null
  let started = 0
  for (const serial of serials) {
    const id = deviceLogId(serial)
    if (logs.value[id]) {
      activeLog.value = id
      continue
    }
    const device = devices.find((d) => d.serial === serial)
    logs.value[id] = {
      name: `${device?.model || device?.product || "Android 设备"} · ${serial}${
        packageName ? ` · ${packageName}` : ""
      }`,
      lines: [],
      filtered: [],
      status: "running",
    }
    logOrder.value.push(id)
    activeLog.value = id
    started++
    // The capture stays pending until stopped; never awaited here.
    startDeviceLogcat(serial, packageName).catch(() => {})
  }
  if (started) {
    showToast(
      packageName
        ? `已开始抓取 ${started} 台设备上「${packageName}」的应用日志`
        : `已开始抓取 ${started} 台设备的整机日志`,
      "success"
    )
  }
}

// Visible lines of the active log: the dedicated filter buffer while a
// filter is active (its matches are never flushed by incoming lines),
// the full tail otherwise.
const displayLines = computed(() => {
  if (!activeLog.value) return [] as string[]
  const entry = logs.value[activeLog.value]
  if (!entry) return [] as string[]
  return logFilter.value.trim() ? entry.filtered : entry.lines
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
  try {
    unlistenDeviceLog = await listenDeviceLog(onDeviceLog)
  } catch {
    // Not running inside Tauri.
  }
})

// KeepAlive 缓存页面：构建队列会被配置区（删项目/改名/记录）级联
// 修改，每次切回时重新加载保持同步。
onActivated(async () => {
  await reload()
  // Pipeline handoff: the config area forwarded a queue for building —
  // open the build-mode dialog for it right away.
  const uuid = pendingBuildRequest.value
  if (uuid) {
    pendingBuildRequest.value = null
    const queue = queues.value.find((q) => q.uuid === uuid)
    if (queue) buildAllQueue.value = queue
  }
})

onUnmounted(() => {
  unlistenLog?.()
  unlistenLog = undefined
  unlistenDeviceLog?.()
  unlistenDeviceLog = undefined
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
  if (!env || buildingUuids.value.size || queueBuildingUuid.value) return
  await runQueueProject(env, target.project, args)
}

function markBuilding(uuid: string) {
  const next = new Set(buildingUuids.value)
  next.add(uuid)
  buildingUuids.value = next
}

function unmarkBuilding(uuid: string) {
  const next = new Set(buildingUuids.value)
  next.delete(uuid)
  buildingUuids.value = next
}

// One project's build inside any flow; its log streams into its own page.
// Resolves to whether the build succeeded. A build-all run passes
// notify=false so the queue raises one summary notification instead of one
// per project.
async function runQueueProject(
  env: string,
  project: BuildProject,
  args: string[],
  notify = true
): Promise<boolean> {
  ensureLog(project.uuid, project.name)
  markBuilding(project.uuid)
  try {
    await runProjectBuild(project.uuid, env, args)
    if (notify) notifySystem("构建完成", `「${project.name}」构建成功`)
    return true
  } catch (e) {
    const message =
      typeof e === "string" ? e : `「${project.name}」构建失败，请查看日志`
    showToast(message)
    if (notify) notifySystem("构建失败", message)
    return false
  } finally {
    unmarkBuilding(project.uuid)
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

// Delete a whole build queue; only the records go, project files stay.
function openDeleteQueue() {
  if (!menu.value) return
  pendingDeleteQueue.value = menu.value.queue
  menu.value = null
}

async function confirmDeleteQueue() {
  const queue = pendingDeleteQueue.value
  if (!queue || deletingQueue.value) return
  deletingQueue.value = true
  try {
    await deleteBuildQueues([queue.uuid])
    pendingDeleteQueue.value = null
    await reload()
    showToast(`已删除队列「${queue.name}」`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除队列失败，请重试")
  } finally {
    deletingQueue.value = false
  }
}

// 清空队列: drops every project card of the queue while the queue itself
// stays. A running build must be stopped first, so no card is pulled away
// mid-build; project files are never touched.
function openClearQueue() {
  if (!menu.value) return
  const queue = menu.value.queue
  menu.value = null
  if (!queue.projects.length) {
    showToast(`队列「${queue.name}」下暂无项目`)
    return
  }
  const building =
    queueBuildingUuid.value === queue.uuid ||
    queue.projects.some((project) => buildingUuids.value.has(project.uuid))
  if (building) {
    showToast("队列正在构建中，请先停止构建再清空")
    return
  }
  pendingClearQueue.value = queue
}

async function confirmClearQueue() {
  const queue = pendingClearQueue.value
  if (!queue || clearingQueue.value) return
  clearingQueue.value = true
  // The cleared cards take their log pages with them (device pages stay).
  const cleared = queue.projects.map((project) => project.uuid)
  try {
    const updated = await clearBuildQueue(queue.uuid)
    replaceQueue(updated)
    pendingClearQueue.value = null
    for (const uuid of cleared) removeLog(uuid)
    showToast(
      `已清空队列「${queue.name}」，移除 ${cleared.length} 个项目卡片`,
      "success"
    )
  } catch (e) {
    showToast(typeof e === "string" ? e : "清空队列失败，请重试")
  } finally {
    clearingQueue.value = false
  }
}

// 全部构建: the mode dialog picks the gradle command and whether the
// queue builds serially (one after another) or in parallel (all at once).
function openBuildAll() {
  if (!menu.value) return
  buildAllQueue.value = menu.value.queue
  menu.value = null
}

async function startBuildAll(args: string[], mode: "serial" | "parallel") {
  const queue = buildAllQueue.value
  buildAllQueue.value = null
  if (!queue) return
  if (!queue.projects.length) {
    showToast(`队列「${queue.name}」下暂无项目`)
    return
  }
  const env = requireGradleEnv()
  if (!env || buildingUuids.value.size || queueBuildingUuid.value) return
  queueBuildingUuid.value = queue.uuid
  stopRequested.value = false
  const total = queue.projects.length
  let executed = 0
  let succeeded = 0
  if (mode === "parallel") {
    // Every project builds at the same time, each into its own log page.
    const results = await Promise.all(
      queue.projects.map((project) => runQueueProject(env, project, args, false))
    )
    executed = results.length
    succeeded = results.filter((ok) => ok).length
  } else {
    for (const project of queue.projects) {
      if (stopRequested.value) {
        showToast("已停止队列构建", "info")
        break
      }
      executed++
      if (await runQueueProject(env, project, args, false)) succeeded++
    }
  }
  queueBuildingUuid.value = ""
  stopRequested.value = false
  showToast(`队列「${queue.name}」构建流程执行完毕`, "success")
  // One summary notification for the whole run.
  const failed = executed - succeeded
  const skipped = total - executed
  notifySystem(
    skipped ? "队列构建已停止" : "全部构建完成",
    `队列「${queue.name}」：成功 ${succeeded} 个${
      failed ? `，失败 ${failed} 个` : ""
    }${skipped ? `，未执行 ${skipped} 个` : ""}`
  )
}

// Stop the running build of one project (kills the whole process tree);
// during a build-all run it also breaks the loop before the next project.
async function onStopProject(project: BuildProject) {
  const entry = logs.value[project.uuid]
  if (entry) {
    const marker = "== 已请求停止构建 =="
    entry.lines.push(marker)
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
        <span
          class="text-muted-foreground shrink-0 text-[clamp(11px,1.25vw,12px)]"
          title="构建产物的目标平台类型，当前仅支持 Android"
        >
          构建类型
        </span>
        <div class="w-[clamp(120px,13vw,170px)]">
          <AppSelect
            v-model="buildType"
            :options="buildTypeOptions"
            aria-label="构建类型"
          />
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span
          class="text-muted-foreground shrink-0 text-[clamp(11px,1.25vw,12px)]"
          title="执行 gradle wrapper 所使用的 Gradle 版本，可在「设置 → 编译」中管理"
        >
          Gradle 环境
        </span>
        <div v-if="gradleEnvOptions.length" class="w-[clamp(120px,13vw,170px)]">
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
              v-for="queue in sortedQueues"
              :key="queue.uuid"
              :queue="queue"
              :building-uuids="[...buildingUuids]"
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
          <div class="flex shrink-0 items-center gap-2">
            <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
              共 {{ logOrder.length }} 个
            </span>
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex h-6 cursor-pointer items-center gap-1 rounded-md bg-muted/60 px-2 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none"
              title="打开日志缓存目录（build/logs）：每次构建与设备抓取的完整日志文件，重启不丢失"
              @click="onOpenLogsDir"
            >
              <FolderOpen class="size-2.5" />
              日志缓存
            </button>
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex h-6 cursor-pointer items-center gap-1 rounded-md bg-muted/60 px-2 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
              title="检测已开启 USB 调试的 Android 设备，按设备分页抓取 logcat 日志"
              :disabled="scanningDevices"
              @click="onCaptureDevices"
            >
              <Loader2 v-if="scanningDevices" class="size-2.5 animate-spin" />
              <Smartphone v-else class="size-2.5" />
              设备日志
            </button>
          </div>
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
            <Smartphone
              v-if="uuid.startsWith('device:')"
              class="text-muted-foreground size-2.5 shrink-0"
            />
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
            placeholder="筛选日志（包含匹配；命中单独缓存 500 条，不会被新日志刷掉）"
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
        <!-- Log body: plain themed text, no colors by design -->
        <div
          ref="logBodyRef"
          class="min-h-0 flex-1 overflow-auto p-2"
          @scroll="onLogScroll"
        >
          <pre
            v-if="activeLog && logs[activeLog]"
            class="text-muted-foreground font-mono text-[clamp(9px,1vw,10px)] leading-relaxed whitespace-pre-wrap break-all"
            >{{ displayLines.join("\n") }}</pre
          >
          <div v-else class="flex h-full items-center justify-center">
            <p
              class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]"
            >
              暂无构建日志，右键队列或项目卡片开始构建，或点右上角「设备日志」抓取手机日志
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
      @build-all-open="openBuildAll"
      @clear-queue="openClearQueue"
      @delete-queue="openDeleteQueue"
    />
    <BuildModeModal
      v-if="buildAllQueue"
      :queue="buildAllQueue"
      @close="buildAllQueue = null"
      @start="startBuildAll"
    />
    <DeviceLogModal
      v-if="deviceModal"
      :devices="deviceModal"
      @close="deviceModal = null"
      @start="onStartDeviceCapture"
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
      confirm-label="删除"
      danger
      :busy="removing"
      @cancel="pendingRemove = null"
      @confirm="confirmRemove"
    />
    <ConfirmDialog
      v-if="pendingClearQueue"
      title="清空构建队列"
      :message="`确定清空队列「${pendingClearQueue.name}」下的全部 ${pendingClearQueue.projects.length} 个项目卡片吗？队列保留，仅移除卡片记录，项目文件不受影响。`"
      confirm-label="清空"
      danger
      :busy="clearingQueue"
      @cancel="pendingClearQueue = null"
      @confirm="confirmClearQueue"
    />
    <ConfirmDialog
      v-if="pendingDeleteQueue"
      title="删除构建队列"
      :message="`确定删除队列「${pendingDeleteQueue.name}」吗？仅删除队列与卡片记录，项目文件不受影响。`"
      confirm-label="删除"
      danger
      :busy="deletingQueue"
      @cancel="pendingDeleteQueue = null"
      @confirm="confirmDeleteQueue"
    />
  </div>
</template>
