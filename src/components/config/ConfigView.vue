<script setup lang="ts">
import { Calendar, CheckSquare, ListPlus, Search, Trash2, X } from "lucide-vue-next"
import { computed, onMounted, ref } from "vue"
import CalendarPicker from "../import/CalendarPicker.vue"
import ConfigDirectoryCard from "./ConfigDirectoryCard.vue"
import ConfigProjectContextMenu from "./ConfigProjectContextMenu.vue"
import ConfigProjectInfoModal from "./ConfigProjectInfoModal.vue"
import ConfigQueueCard from "./ConfigQueueCard.vue"
import ConfigQueueContextMenu from "./ConfigQueueContextMenu.vue"
import CreateConfigQueueModal from "./CreateConfigQueueModal.vue"
import CreateDiskProjectModal from "./CreateDiskProjectModal.vue"
import PickImportedProjectModal from "./PickImportedProjectModal.vue"
import SelectTemplateModal from "./SelectTemplateModal.vue"
import ConfirmDialog from "../import/ConfirmDialog.vue"
import type { AndroidProject } from "@/lib/android"
import {
  addConfigProject,
  deleteConfigProjects,
  deleteConfigQueues,
  executeConfigProject,
  listConfigQueues,
  recordAllConfigProjects,
  recordConfigProject,
  removeConfigProject,
  saveConfigTemplate,
  startConfigProject,
  type ConfigProject,
  type ConfigQueue,
} from "@/lib/config"
import { showToast } from "@/lib/toast"

const queues = ref<ConfigQueue[]>([])

// Search keyword; the actual project filtering is not wired up yet.
const projectKeyword = ref("")

// Config-date filter (YYYY-MM-DD); shared by both directories. Queues
// match their creation date, directory projects their config (start)
// time.
const timeFilter = ref("")
// Themed calendar popover state (anchor position of the trigger).
const calendar = ref<{ x: number; y: number } | null>(null)

function openCalendar(event: MouseEvent) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  calendar.value = { x: rect.left, y: rect.bottom + 4 }
}

function onPickDate(date: string) {
  timeFilter.value = date
  calendar.value = null
}

// Create-queue modal state.
const showCreate = ref(false)
// Right-click context menu state.
const menu = ref<{ x: number; y: number; queue: ConfigQueue } | null>(null)
// Queue currently receiving projects through the pick modal.
const pickQueue = ref<ConfigQueue | null>(null)
// Sub card waiting for remove confirmation.
const pendingRemove = ref<{ queue: ConfigQueue; project: ConfigProject } | null>(null)
const removing = ref(false)
// Project waiting for the launch (完善配置) confirmation; the uuid of the
// one currently executing drives the card's loading state.
const pendingExecute = ref<ConfigProject | null>(null)
const executingUuid = ref("")
// True while the disk directory dialog/attach is running.
const attachingDisk = ref(false)
// Naming modal state after picking a directory from disk.
const diskPick = ref<{ queueUuid: string; pickedPath: string } | null>(null)
// Queue uuid currently receiving projects (shows the loading indicator).
const addingUuid = ref("")
// Project (and its queue) whose info modal is open.
const infoTarget = ref<{ queueUuid: string; project: ConfigProject } | null>(null)
// Right-click context menu state of a sub project card.
const projectMenu = ref<{
  x: number
  y: number
  queue: ConfigQueue
  project: ConfigProject
} | null>(null)
// Sub project currently picking a template through the modal.
const templateTarget = ref<{ queue: ConfigQueue; project: ConfigProject } | null>(null)
const savingTemplate = ref(false)

// Batch selection mode: “queues” or “projects”, empty when inactive.
const selectTarget = ref<"" | "queues" | "projects">("")
const selectedQueues = ref<Set<string>>(new Set())
const selectedProjects = ref<Set<string>>(new Set())

// Generic destructive-delete confirmation (single/batch queues, single/
// batch directory projects). The dialog runs `run` after the user
// confirms.
const confirm = ref<{
  title: string
  message: string
  run: () => Promise<void>
} | null>(null)
const confirming = ref(false)

// Configured (started) projects of every queue, shown in the project
// directory on the right.
const startedProjects = computed(() =>
  queues.value.flatMap((queue) => queue.projects.filter((p) => p.started))
)

// Queue list filtered by the shared config-date filter (creation date).
const filteredQueues = computed(() =>
  !timeFilter.value
    ? queues.value
    : queues.value.filter((q) => q.createdAt.startsWith(timeFilter.value))
)

// Directory projects filtered by their config (start) time.
const filteredStarted = computed(() =>
  !timeFilter.value
    ? startedProjects.value
    : startedProjects.value.filter((p) =>
        (p.startedAt ?? "").startsWith(timeFilter.value)
      )
)

const selectedCount = computed(() =>
  selectTarget.value === "queues"
    ? selectedQueues.value.size
    : selectedProjects.value.size
)

const allSelected = computed(() =>
  selectTarget.value === "queues"
    ? filteredQueues.value.length > 0 &&
      filteredQueues.value.every((q) => selectedQueues.value.has(q.uuid))
    : filteredStarted.value.length > 0 &&
      filteredStarted.value.every((project) =>
        selectedProjects.value.has(project.uuid)
      )
)

async function reload() {
  queues.value = await listConfigQueues()
}

onMounted(reload)

function replaceQueue(updated: ConfigQueue) {
  queues.value = queues.value.map((q) =>
    q.uuid === updated.uuid ? updated : q
  )
}

function onQueueSaved(queue: ConfigQueue) {
  queues.value.push(queue)
  showToast(`配置队列「${queue.name}」创建成功`, "success")
}

function onContextMenu(queue: ConfigQueue, event: MouseEvent) {
  menu.value = { x: event.clientX, y: event.clientY, queue }
}

// --- Option 1: configure from an imported project ---------------------------

function openPickImported() {
  if (!menu.value) return
  pickQueue.value = menu.value.queue
  menu.value = null
}

// Attach every picked imported project as a sub card of the queue. The
// pick modal stays open with its loading state until the copies finish.
async function onPickConfirm(picks: AndroidProject[]) {
  const queue = pickQueue.value
  if (!queue || !picks.length || addingUuid.value) return
  addingUuid.value = queue.uuid
  try {
    // Skip projects already attached to this queue (compared by package
    // name), the backend rejects duplicates too.
    const existing = new Set(
      queue.projects
        .filter((p) => p.source === "imported")
        .map((p) => p.packageName)
    )
    let added = 0
    let error = ""
    for (const project of picks) {
      if (existing.has(project.packageName)) continue
      try {
        const updated = await addConfigProject(queue.uuid, {
          name: project.appName,
          source: "imported",
          packageName: project.packageName,
          rootPath: project.location ?? project.rootPath,
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
  } finally {
    addingUuid.value = ""
    pickQueue.value = null
  }
}

// --- Option 2: configure from a directory picked on disk --------------------

async function pickFromDisk() {
  if (!menu.value || attachingDisk.value) return
  const queue = menu.value.queue
  menu.value = null
  attachingDisk.value = true
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择项目目录",
    })
    if (!selected) return
    // The picked directory's contents are copied by the backend under the
    // user-entered config name (the naming modal collects it).
    diskPick.value = { queueUuid: queue.uuid, pickedPath: selected as string }
  } catch (e) {
    showToast(typeof e === "string" ? e : "打开目录选择失败")
  } finally {
    attachingDisk.value = false
  }
}

function onDiskSaved(updated: ConfigQueue, name: string) {
  replaceQueue(updated)
  showToast(`项目「${name}」创建成功`, "success")
}

// --- Batch selection -------------------------------------------------------

function enterSelectMode(target: "queues" | "projects") {
  selectTarget.value = target
  selectedQueues.value = new Set()
  selectedProjects.value = new Set()
}

function exitSelectMode() {
  selectTarget.value = ""
  selectedQueues.value = new Set()
  selectedProjects.value = new Set()
}

function toggleQueue(uuid: string) {
  const next = new Set(selectedQueues.value)
  next.has(uuid) ? next.delete(uuid) : next.add(uuid)
  selectedQueues.value = next
}

function toggleProject(uuid: string) {
  const next = new Set(selectedProjects.value)
  next.has(uuid) ? next.delete(uuid) : next.add(uuid)
  selectedProjects.value = next
}

function toggleSelectAll() {
  if (selectTarget.value === "queues") {
    selectedQueues.value = allSelected.value
      ? new Set()
      : new Set(filteredQueues.value.map((q) => q.uuid))
  } else {
    selectedProjects.value = allSelected.value
      ? new Set()
      : new Set(filteredStarted.value.map((project) => project.uuid))
  }
}

// --- Queue deletion ---------------------------------------------------------

function onDeleteQueue(queue: ConfigQueue) {
  confirm.value = {
    title: "删除队列",
    message: `确定删除队列「${queue.name}」吗？队列下的项目会随队列删除（已复制的文件保留），删除后不可恢复。`,
    run: async () => {
      await deleteConfigQueues([queue.uuid])
      await reload()
      showToast("队列已删除", "success")
    },
  }
}

function openDeleteQueue() {
  if (!menu.value) return
  const queue = menu.value.queue
  menu.value = null
  onDeleteQueue(queue)
}

function requestBatchDelete() {
  if (selectTarget.value === "queues") {
    const uuids = [...selectedQueues.value]
    if (!uuids.length) return
    confirm.value = {
      title: "批量删除队列",
      message: `确定删除所选 ${uuids.length} 个队列吗？队列下的项目会随队列删除（已复制的文件保留），删除后不可恢复。`,
      run: async () => {
        await deleteConfigQueues(uuids)
        exitSelectMode()
        await reload()
        showToast("所选队列已删除", "success")
      },
    }
  } else {
    const uuids = [...selectedProjects.value]
    if (!uuids.length) return
    confirm.value = {
      title: "批量删除项目",
      message: `确定删除所选 ${uuids.length} 个项目吗？对应的配置目录也会一并删除，删除后不可恢复。`,
      run: async () => {
        await deleteConfigProjects(uuids)
        exitSelectMode()
        await reload()
        showToast("所选项目已删除", "success")
      },
    }
  }
}

async function runConfirm() {
  if (!confirm.value || confirming.value) return
  confirming.value = true
  try {
    await confirm.value.run()
    confirm.value = null
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    confirming.value = false
  }
}

// Directory card delete: removes the project record and its copied config
// directory.
function onDeleteDirectoryProject(project: ConfigProject) {
  confirm.value = {
    title: "删除项目",
    message: `确定删除项目「${project.name}」吗？对应的配置目录也会一并删除，删除后不可恢复。`,
    run: async () => {
      await deleteConfigProjects([project.uuid])
      await reload()
      showToast("项目已删除", "success")
    },
  }
}

// Open the info modal of a directory project; the queue is looked up so
// the modal can call the update/reload commands.
function onEditDirectoryProject(project: ConfigProject) {
  const queue = queues.value.find((q) =>
    q.projects.some((p) => p.uuid === project.uuid)
  )
  if (!queue) return
  infoTarget.value = { queueUuid: queue.uuid, project }
}

// --- Sub card removal --------------------------------------------------------

function onDeleteProject(queue: ConfigQueue, project: ConfigProject) {
  pendingRemove.value = { queue, project }
}

async function confirmRemove() {
  const target = pendingRemove.value
  if (!target || removing.value) return
  removing.value = true
  try {
    const updated = await removeConfigProject(
      target.queue.uuid,
      target.project.uuid
    )
    replaceQueue(updated)
    pendingRemove.value = null
    showToast("已从队列删除卡片", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    removing.value = false
  }
}

// --- Launch (kernel injection) ----------------------------------------------

function onExecuteProject(project: ConfigProject) {
  pendingExecute.value = project
}

// After the user confirms: copy the template's code folder into the
// project's config directory, then run the argument kernel followed by
// the code kernel using the package-named parameter JSON.
async function confirmExecute() {
  const target = pendingExecute.value
  if (!target || executingUuid.value) return
  executingUuid.value = target.uuid
  try {
    const summary = await executeConfigProject(target.uuid)
    pendingExecute.value = null
    // The backend marks the project as code-copied; refresh local state.
    await reload()
    showToast(summary, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "启动失败，请重试")
  } finally {
    executingUuid.value = ""
  }
}

// --- Sub project config actions ---------------------------------------------

function onProjectContextMenu(
  queue: ConfigQueue,
  project: ConfigProject,
  event: MouseEvent
) {
  projectMenu.value = { x: event.clientX, y: event.clientY, queue, project }
}

function openSelectTemplate() {
  if (!projectMenu.value) return
  templateTarget.value = {
    queue: projectMenu.value.queue,
    project: projectMenu.value.project,
  }
  projectMenu.value = null
}

// 选择配置模板后开始配置: save the template (recording the modify-config
// time) and optionally start the configuration right away.
async function onTemplateSave(templateName: string, start: boolean) {
  const target = templateTarget.value
  if (!target || savingTemplate.value) return
  savingTemplate.value = true
  try {
    let updated = await saveConfigTemplate(
      target.queue.uuid,
      target.project.uuid,
      templateName
    )
    if (start) {
      updated = await startConfigProject(target.queue.uuid, target.project.uuid)
    }
    replaceQueue(updated)
    templateTarget.value = null
    showToast(
      start
        ? `项目「${target.project.name}」已开始配置`
        : `项目「${target.project.name}」的模板已保存`,
      "success"
    )
  } catch (e) {
    showToast(typeof e === "string" ? e : "保存失败，请重试")
  } finally {
    savingTemplate.value = false
  }
}

// --- Record actions ----------------------------------------------------------
// 开始配置 is only reachable through the template modal's 保存并开始配置.

// 记录项目: copy the project's contents into the config area (imported
// projects from the import area, disk projects from their disk directory).
async function onRecordProject(queue: ConfigQueue, project: ConfigProject) {
  projectMenu.value = null
  if (addingUuid.value) return
  addingUuid.value = queue.uuid
  try {
    const updated = await recordConfigProject(queue.uuid, project.uuid)
    replaceQueue(updated)
    showToast(`项目「${project.name}」记录完成`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "记录失败，请重试")
  } finally {
    addingUuid.value = ""
  }
}

// 记录全部项目: record every project of the queue at once.
async function onRecordAll() {
  if (!menu.value || addingUuid.value) return
  const queue = menu.value.queue
  menu.value = null
  if (!queue.projects.length) {
    showToast(`队列「${queue.name}」下暂无项目`)
    return
  }
  addingUuid.value = queue.uuid
  try {
    const updated = await recordAllConfigProjects(queue.uuid)
    replaceQueue(updated)
    showToast(`队列「${queue.name}」记录完成`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "记录失败，请重试")
  } finally {
    addingUuid.value = ""
  }
}
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      配置区
    </h1>
    <div class="flex shrink-0 items-center gap-3">
      <!-- Project search: same look as the import page; filtering is TBD. -->
      <div class="relative w-[clamp(140px,16vw,220px)]">
        <Search
          class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
        />
        <input
          v-model="projectKeyword"
          type="text"
          placeholder="搜索项目，支持名称或包名"
          class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input pr-8 pl-8 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
        />
        <button
          v-if="projectKeyword"
          type="button"
          class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1.5 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
          aria-label="清空项目搜索"
          @click="projectKeyword = ''"
        >
          <X class="size-3" />
        </button>
      </div>
      <!-- Config-date filter next to the search, applied to both
           directories at once; opens the themed CalendarPicker popover. -->
      <div class="relative">
        <button
          type="button"
          class="bg-background hover:bg-accent/40 flex h-8 w-[clamp(120px,13vw,170px)] cursor-pointer items-center gap-2 rounded-lg border border-input pr-7 pl-2.5 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
          aria-label="按配置日期筛选"
          title="按配置日期筛选（两个目录同时生效）"
          @click="openCalendar"
        >
          <Calendar class="text-muted-foreground size-3.5 shrink-0" />
          <span
            class="truncate"
            :class="timeFilter ? 'text-foreground' : 'text-muted-foreground'"
          >
            {{ timeFilter || "按配置日期筛选" }}
          </span>
        </button>
        <button
          v-if="timeFilter"
          type="button"
          class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1.5 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
          aria-label="清除日期筛选"
          @click="timeFilter = ''"
        >
          <X class="size-3" />
        </button>
      </div>
      <!-- Right cluster: mode-dependent actions -->
      <div class="ml-auto flex shrink-0 items-center gap-3">
        <!-- Batch selection actions -->
        <template v-if="selectTarget">
          <p
            class="text-muted-foreground shrink-0 truncate text-[clamp(10px,1.1vw,11px)]"
          >
            已选 {{ selectedCount }} 项（{{
              selectTarget === "queues" ? "队列" : "项目"
            }}）
          </p>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="toggleSelectAll"
          >
            <CheckSquare class="size-3.5" />
            {{ allSelected ? "取消全选" : "全选" }}
          </button>
          <button
            type="button"
            class="text-destructive hover:bg-destructive/10 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-destructive/5 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="!selectedCount"
            @click="requestBatchDelete"
          >
            <Trash2 class="size-3.5" />
            删除所选{{ selectedCount ? `(${selectedCount})` : "" }}
          </button>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="exitSelectMode"
          >
            完成
          </button>
        </template>
        <!-- Normal actions -->
        <template v-else>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="!queues.length"
            @click="enterSelectMode('queues')"
          >
            <Trash2 class="size-3.5" />
            批量删除队列
          </button>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="!startedProjects.length"
            @click="enterSelectMode('projects')"
          >
            <Trash2 class="size-3.5" />
            批量删除项目
          </button>
          <!-- Primary action, same style as the import page's
               “创建导入队列” button. -->
          <button
            type="button"
            class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="showCreate = true"
          >
            <ListPlus class="size-3.5" />
            创建配置队列
          </button>
        </template>
      </div>
    </div>
    <div class="flex min-h-0 flex-1 gap-3">
      <!-- Queue directory: one quarter of the page width with its own
           scrollbar, mirroring the import page. -->
      <aside
        class="flex min-h-0 w-1/4 shrink-0 flex-col rounded-xl border border-border"
      >
        <header
          class="flex shrink-0 items-center justify-between px-3 py-2"
        >
          <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">队列目录</h2>
          <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
            <template v-if="timeFilter">
              匹配 {{ filteredQueues.length }} / {{ queues.length }}
            </template>
            <template v-else>共 {{ queues.length }} 个</template>
          </span>
        </header>
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
          <template v-if="filteredQueues.length">
            <ConfigQueueCard
              v-for="queue in filteredQueues"
              :key="queue.uuid"
              :queue="queue"
              :adding="addingUuid === queue.uuid"
              :select-mode="selectTarget === 'queues'"
              :selected="selectedQueues.has(queue.uuid)"
              @delete-project="onDeleteProject(queue, $event)"
              @contextmenu="onContextMenu(queue, $event)"
              @project-contextmenu="(project, event) => onProjectContextMenu(queue, project, event)"
              @toggle-select="toggleQueue(queue.uuid)"
            />
          </template>
          <div v-else class="flex flex-1 items-center justify-center">
            <p class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]">
              {{
                queues.length
                  ? "未匹配到队列"
                  : "暂无队列，点击右上角创建"
              }}
            </p>
          </div>
        </div>
      </aside>
      <!-- Project directory: fills the remaining width, same design as
           the queue directory. -->
      <aside
        class="flex min-h-0 min-w-0 flex-1 flex-col rounded-xl border border-border"
      >
        <header
          class="flex shrink-0 items-center justify-between px-3 py-2"
        >
          <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">项目目录</h2>
          <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
            <template v-if="timeFilter">
              匹配 {{ filteredStarted.length }} / {{ startedProjects.length }}
            </template>
            <template v-else>共 {{ startedProjects.length }} 个</template>
          </span>
        </header>
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
          <template v-if="filteredStarted.length">
            <ConfigDirectoryCard
              v-for="project in filteredStarted"
              :key="project.uuid"
              :project="project"
              :select-mode="selectTarget === 'projects'"
              :selected="selectedProjects.has(project.uuid)"
              :executing="executingUuid === project.uuid"
              @edit="onEditDirectoryProject(project)"
              @execute="onExecuteProject(project)"
              @delete="onDeleteDirectoryProject(project)"
              @toggle-select="toggleProject(project.uuid)"
            />
          </template>
          <div v-else class="flex flex-1 items-center justify-center">
            <p class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]">
              {{
                startedProjects.length
                  ? "未匹配到项目"
                  : "暂无项目，开始配置后在此显示"
              }}
            </p>
          </div>
        </div>
      </aside>
    </div>
    <CreateConfigQueueModal
      v-if="showCreate"
      @close="showCreate = false"
      @saved="onQueueSaved"
    />
    <ConfigQueueContextMenu
      v-if="menu"
      :x="menu.x"
      :y="menu.y"
      :queue="menu.queue"
      @close="menu = null"
      @pick-imported="openPickImported"
      @pick-disk="pickFromDisk"
      @record-all="onRecordAll"
      @delete-queue="openDeleteQueue"
    />
    <PickImportedProjectModal
      v-if="pickQueue"
      @close="pickQueue = null"
      @confirm="onPickConfirm"
    />
    <CreateDiskProjectModal
      v-if="diskPick"
      :queue-uuid="diskPick.queueUuid"
      :picked-path="diskPick.pickedPath"
      @close="diskPick = null"
      @saved="onDiskSaved"
    />
    <ConfigProjectContextMenu
      v-if="projectMenu"
      :x="projectMenu.x"
      :y="projectMenu.y"
      :project="projectMenu.project"
      @close="projectMenu = null"
      @pick-template="openSelectTemplate"
      @record="onRecordProject(projectMenu.queue, projectMenu.project)"
    />
    <SelectTemplateModal
      v-if="templateTarget"
      :project-name="templateTarget.project.name"
      @close="templateTarget = null"
      @save="onTemplateSave"
    />
    <ConfigProjectInfoModal
      v-if="infoTarget"
      :project="infoTarget.project"
      :queue-uuid="infoTarget.queueUuid"
      @close="infoTarget = null"
      @saved="replaceQueue"
    />
    <ConfirmDialog
      v-if="confirm"
      :title="confirm.title"
      :message="confirm.message"
      :busy="confirming"
      @cancel="confirm = null"
      @confirm="runConfirm"
    />
    <ConfirmDialog
      v-if="pendingRemove"
      title="从队列删除卡片"
      :message="`确定将项目「${pendingRemove.project.name}」从队列「${pendingRemove.queue.name}」中删除吗？项目文件不会被删除。`"
      :busy="removing"
      @cancel="pendingRemove = null"
      @confirm="confirmRemove"
    />
    <ConfirmDialog
      v-if="pendingExecute"
      title="完善配置"
      :message="
        pendingExecute.codeCopied
          ? '模板 code 内容已复制过，本次启动仅执行 argument 与 code 内核注入。是否继续？'
          : `是否完善配置？确认后将模板「${pendingExecute.templateName ?? ''}」的 code 内容复制到项目配置目录（同名文件直接覆盖），并依次执行 argument 与 code 内核注入。`
      "
      confirm-label="确认"
      :busy="executingUuid !== ''"
      @cancel="pendingExecute = null"
      @confirm="confirmExecute"
    />
    <CalendarPicker
      v-if="calendar"
      :x="calendar.x"
      :y="calendar.y"
      :model-value="timeFilter"
      @close="calendar = null"
      @pick="onPickDate"
    />
  </div>
</template>
