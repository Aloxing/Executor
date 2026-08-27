<script setup lang="ts">
import { Calendar, CheckSquare, ListPlus, Search, Trash2, X } from "lucide-vue-next"
import { computed, onMounted, ref } from "vue"
import AndroidProjectModal from "./AndroidProjectModal.vue"
import CalendarPicker from "./CalendarPicker.vue"
import ConfirmDialog from "./ConfirmDialog.vue"
import CreateQueueModal from "./CreateQueueModal.vue"
import DeleteProjectDialog from "./DeleteProjectDialog.vue"
import ImportProjectCard from "./ImportProjectCard.vue"
import QueueCard from "./QueueCard.vue"
import QueueContextMenu from "./QueueContextMenu.vue"
import {
  deleteAndroidProject,
  deleteAndroidProjects,
  detachAndroidProject,
  importAndroidProjects,
  listAndroidProjects,
  type AndroidProject,
} from "@/lib/android"
import { deleteQueues, listImportQueues, type ImportQueue } from "@/lib/queues"
import { showToast } from "@/lib/toast"

const queues = ref<ImportQueue[]>([])
const projects = ref<AndroidProject[]>([])

// Create-queue modal state.
const showCreate = ref(false)
// Right-click context menu state.
const menu = ref<{ x: number; y: number; queue: ImportQueue } | null>(null)
// Add/edit Android project modal state.
const projectModal = ref<{
  queueUuid: string
  initial?: AndroidProject
} | null>(null)
// Project waiting for delete confirmation (import directory, destructive).
const pendingDelete = ref<AndroidProject | null>(null)
// Project waiting for detach confirmation (queue sub card, keeps data).
const pendingDetach = ref<AndroidProject | null>(null)
const deleting = ref(false)
// Queue uuid currently running its import action.
const importingUuid = ref("")

// Batch selection mode: “queues” or “projects”, empty when inactive.
const selectTarget = ref<"" | "queues" | "projects">("")
const selectedQueues = ref<Set<string>>(new Set())
const selectedProjects = ref<Set<string>>(new Set())

// Generic destructive-delete confirmation (single/batch queues, batch
// projects). The dialog runs `run` after the user confirms.
const confirm = ref<{
  title: string
  message: string
  run: () => Promise<void>
} | null>(null)
const confirming = ref(false)

const selectedCount = computed(() =>
  selectTarget.value === "queues"
    ? selectedQueues.value.size
    : selectedProjects.value.size
)

// --- Search filtering -------------------------------------------------------

const projectKeyword = ref("")
// Creation-date filter (YYYY-MM-DD); shared by both directories.
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

function matchesTime(createdAt: string): boolean {
  return !timeFilter.value || createdAt.startsWith(timeFilter.value)
}

// Queue list filtered by the shared creation-date filter.
const filteredQueues = computed(() =>
  queues.value.filter((q) => matchesTime(q.createdAt))
)

// Project search matches the app name and the package name.
const filteredProjects = computed(() => {
  const kw = projectKeyword.value.trim().toLowerCase()
  return projects.value.filter((p) => {
    if (!matchesTime(p.createdAt)) return false
    if (!kw) return true
    return (
      p.packageName.toLowerCase().includes(kw) ||
      p.appName.toLowerCase().includes(kw)
    )
  })
})

const allSelected = computed(() =>
  selectTarget.value === "queues"
    ? filteredQueues.value.length > 0 &&
      filteredQueues.value.every((q) => selectedQueues.value.has(q.uuid))
    : filteredProjects.value.length > 0 &&
      filteredProjects.value.every((p) =>
        selectedProjects.value.has(p.packageName)
      )
)

function projectsOf(queue: ImportQueue): AndroidProject[] {
  return projects.value.filter((p) => p.queueUuid === queue.uuid)
}

async function reload() {
  queues.value = await listImportQueues()
  projects.value = await listAndroidProjects()
}

onMounted(reload)

function onQueueSaved(queue: ImportQueue) {
  queues.value.push(queue)
  showToast(`导入队列「${queue.name}」创建成功`, "success")
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

function toggleProject(packageName: string) {
  const next = new Set(selectedProjects.value)
  next.has(packageName) ? next.delete(packageName) : next.add(packageName)
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
      : new Set(filteredProjects.value.map((p) => p.packageName))
  }
}

// --- Queue deletion ---------------------------------------------------------

function onDeleteQueue(queue: ImportQueue) {
  confirm.value = {
    title: "删除队列",
    message: `确定删除队列「${queue.name}」吗？队列下的项目会保留在项目目录中，删除后不可恢复。`,
    run: async () => {
      await deleteQueues([queue.uuid])
      await reload()
      showToast("队列已删除", "success")
    },
  }
}

function requestBatchDelete() {
  if (selectTarget.value === "queues") {
    const uuids = [...selectedQueues.value]
    if (!uuids.length) return
    confirm.value = {
      title: "批量删除队列",
      message: `确定删除所选 ${uuids.length} 个队列吗？队列下的项目会保留在项目目录中，删除后不可恢复。`,
      run: async () => {
        await deleteQueues(uuids)
        exitSelectMode()
        await reload()
        showToast("所选队列已删除", "success")
      },
    }
  } else {
    const names = [...selectedProjects.value]
    if (!names.length) return
    confirm.value = {
      title: "批量删除项目",
      message: `确定删除所选 ${names.length} 个项目吗？对应的包名文件夹也会一并删除，删除后不可恢复。`,
      run: async () => {
        await deleteAndroidProjects(names)
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

function onContextMenu(queue: ImportQueue, event: MouseEvent) {
  menu.value = { x: event.clientX, y: event.clientY, queue }
}

function openAddProject() {
  if (!menu.value) return
  projectModal.value = { queueUuid: menu.value.queue.uuid }
  menu.value = null
}

function openDeleteQueue() {
  if (!menu.value) return
  const queue = menu.value.queue
  menu.value = null
  onDeleteQueue(queue)
}

async function onProjectSaved(project: AndroidProject) {
  await reload()
  showToast(`Android 项目「${project.appName}」保存成功`, "success")
}

async function onProjectReloaded(project: AndroidProject) {
  await reload()
  showToast(`项目「${project.appName}」已重新加载`, "success")
}

function onEditProject(project: AndroidProject) {
  projectModal.value = { queueUuid: project.queueUuid, initial: project }
}

// Import directory card delete: destructive delete of the record and the
// package folder.
function onDeleteProject(project: AndroidProject) {
  pendingDelete.value = project
}

// Sub card delete: detach the card from its queue, keeping the record and
// the package folder (the project stays in the import directory).
function onDetachProject(project: AndroidProject) {
  pendingDetach.value = project
}

async function confirmDetach() {
  const target = pendingDetach.value
  if (!target || deleting.value) return
  deleting.value = true
  try {
    await detachAndroidProject(target.packageName)
    pendingDetach.value = null
    await reload()
    showToast("已从队列删除卡片", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    deleting.value = false
  }
}

async function confirmDelete() {
  const target = pendingDelete.value
  if (!target || deleting.value) return
  deleting.value = true
  try {
    await deleteAndroidProject(target.packageName)
    pendingDelete.value = null
    await reload()
    showToast("Android 项目已删除", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    deleting.value = false
  }
}

// Only this action copies the recorded root directories into the
// `package/<package name>` folders; adding a project merely records it.
async function onImport(queue: ImportQueue) {
  if (importingUuid.value) return
  importingUuid.value = queue.uuid
  // Show the transient 导入中 state while the copy runs.
  projects.value = projects.value.map((p) =>
    p.queueUuid === queue.uuid && p.importStatus === "pending"
      ? { ...p, importStatus: "importing" as const }
      : p
  )
  try {
    await importAndroidProjects(queue.uuid)
    showToast(`队列「${queue.name}」导入完成`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "导入失败，请重试")
  } finally {
    importingUuid.value = ""
    await reload()
  }
}
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      导入区
    </h1>
    <div class="flex shrink-0 items-center gap-3">
      <!-- Project search -->
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
      <!-- Creation-date filter next to the search, applied to both
           directories at once; opens the themed CalendarPicker popover. -->
      <div class="relative">
        <button
          type="button"
          class="bg-background hover:bg-accent/40 flex h-8 w-[clamp(120px,13vw,170px)] cursor-pointer items-center gap-2 rounded-lg border border-input pr-7 pl-2.5 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
          aria-label="按创建日期筛选"
          title="按创建日期筛选（两个目录同时生效）"
          @click="openCalendar"
        >
          <Calendar class="text-muted-foreground size-3.5 shrink-0" />
          <span
            class="truncate"
            :class="timeFilter ? 'text-foreground' : 'text-muted-foreground'"
          >
            {{ timeFilter || "按创建日期筛选" }}
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
          :disabled="!projects.length"
          @click="enterSelectMode('projects')"
        >
          <Trash2 class="size-3.5" />
          批量删除项目
        </button>
        <!-- Primary action, same style as the templates page's
             “创建模板类型” button. -->
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="showCreate = true"
        >
          <ListPlus class="size-3.5" />
          创建导入队列
        </button>
        </template>
      </div>
    </div>
    <div class="flex min-h-0 flex-1 gap-3">
      <!-- Queue directory: one quarter of the page width with its own
           scrollbar, so many cards never scroll the page itself. -->
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
            <QueueCard
              v-for="queue in filteredQueues"
              :key="queue.uuid"
              :queue="queue"
              :projects="projectsOf(queue)"
              :importing="importingUuid === queue.uuid"
              :select-mode="selectTarget === 'queues'"
              :selected="selectedQueues.has(queue.uuid)"
              @import="onImport(queue)"
              @toggle-select="toggleQueue(queue.uuid)"
              @delete-project="onDetachProject"
              @contextmenu="onContextMenu(queue, $event)"
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
      <!-- Project directory: fills the remaining width, same design as the
           queue directory; holds the project cards of every type. -->
      <aside
        class="flex min-h-0 min-w-0 flex-1 flex-col rounded-xl border border-border"
      >
        <header
          class="flex shrink-0 items-center justify-between px-3 py-2"
        >
          <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">项目目录</h2>
          <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
            <template v-if="projectKeyword.trim() || timeFilter">
              匹配 {{ filteredProjects.length }} / {{ projects.length }}
            </template>
            <template v-else>共 {{ projects.length }} 个</template>
          </span>
        </header>
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
          <template v-if="filteredProjects.length">
            <ImportProjectCard
              v-for="project in filteredProjects"
              :key="project.packageName"
              :project="project"
              :select-mode="selectTarget === 'projects'"
              :selected="selectedProjects.has(project.packageName)"
              @edit="onEditProject(project)"
              @delete="onDeleteProject(project)"
              @toggle-select="toggleProject(project.packageName)"
            />
          </template>
          <div v-else class="flex flex-1 items-center justify-center">
            <p class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]">
              {{
                projects.length
                  ? "未匹配到项目"
                  : "暂无项目，在左侧队列卡片上右键添加"
              }}
            </p>
          </div>
        </div>
      </aside>
    </div>
    <CreateQueueModal
      v-if="showCreate"
      @close="showCreate = false"
      @saved="onQueueSaved"
    />
    <QueueContextMenu
      v-if="menu"
      :x="menu.x"
      :y="menu.y"
      :queue="menu.queue"
      @close="menu = null"
      @add-android="openAddProject"
      @delete-queue="openDeleteQueue"
    />
    <AndroidProjectModal
      v-if="projectModal"
      :queue-uuid="projectModal.queueUuid"
      :initial="projectModal.initial"
      @close="projectModal = null"
      @saved="onProjectSaved"
      @reloaded="onProjectReloaded"
    />
    <DeleteProjectDialog
      v-if="pendingDelete"
      :project="pendingDelete"
      @cancel="pendingDelete = null"
      @confirm="confirmDelete"
    />
    <DeleteProjectDialog
      v-if="pendingDetach"
      :project="pendingDetach"
      mode="detach"
      @cancel="pendingDetach = null"
      @confirm="confirmDetach"
    />
    <ConfirmDialog
      v-if="confirm"
      :title="confirm.title"
      :message="confirm.message"
      :busy="confirming"
      @cancel="confirm = null"
      @confirm="runConfirm"
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
