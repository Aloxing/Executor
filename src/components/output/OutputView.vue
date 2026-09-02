<script setup lang="ts">
import { Calendar, CheckSquare, Search, Trash2, X } from "lucide-vue-next"
import { computed, onActivated, onMounted, ref } from "vue"
import AppSelect from "../AppSelect.vue"
import CalendarPicker from "../import/CalendarPicker.vue"
import ConfirmDialog from "../import/ConfirmDialog.vue"
import OutputCard from "./OutputCard.vue"
import {
  copyOutputFile,
  listOutputs,
  removeOutputFile,
  removeOutputs,
  type OutputFile,
  type OutputRecord,
} from "@/lib/output"
import { useShortcut } from "@/lib/shortcuts"
import { showToast } from "@/lib/toast"

const records = ref<OutputRecord[]>([])

// Search: matches the project name, package name, template name and the
// artifact file names.
const keyword = ref("")
const searchInput = ref<HTMLInputElement | null>(null)
useShortcut("search", () => {
  searchInput.value?.focus()
})

// Record-date filter (YYYY-MM-DD) with the themed calendar popover.
const timeFilter = ref("")
const calendar = ref<{ x: number; y: number } | null>(null)

function openCalendar(event: MouseEvent) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  calendar.value = { x: rect.left, y: rect.bottom + 4 }
}

function onPickDate(date: string) {
  timeFilter.value = date
  calendar.value = null
}

// Template tag filter: the distinct template names of the current cards;
// 「无模板」 collects the direct disk builds without config info.
const NO_TEMPLATE = "__none__"
const templateFilter = ref("")

const templateFilterOptions = computed(() => {
  const options = [{ value: "", label: "全部模板" }]
  const names = new Set<string>()
  let hasNoTemplate = false
  for (const record of records.value) {
    if (record.templateName) names.add(record.templateName)
    else hasNoTemplate = true
  }
  for (const name of [...names].sort()) {
    options.push({ value: name, label: name })
  }
  if (hasNoTemplate) options.push({ value: NO_TEMPLATE, label: "无模板" })
  return options
})

const filtered = computed(() => {
  let list = records.value
  const kw = keyword.value.trim().toLowerCase()
  if (kw) {
    list = list.filter(
      (r) =>
        r.projectName.toLowerCase().includes(kw) ||
        (r.packageName ?? "").toLowerCase().includes(kw) ||
        (r.templateName ?? "").toLowerCase().includes(kw) ||
        r.files.some((f) => f.name.toLowerCase().includes(kw))
    )
  }
  if (timeFilter.value) {
    list = list.filter((r) => r.createdAt.startsWith(timeFilter.value))
  }
  if (templateFilter.value === NO_TEMPLATE) {
    list = list.filter((r) => !r.templateName)
  } else if (templateFilter.value) {
    list = list.filter((r) => r.templateName === templateFilter.value)
  }
  return list
})

// --- Batch selection -----------------------------------------------------------

const selectMode = ref(false)
const selected = ref<Set<string>>(new Set())

const selectedCount = computed(() => selected.value.size)
const allSelected = computed(
  () =>
    filtered.value.length > 0 &&
    filtered.value.every((r) => selected.value.has(r.uuid))
)

function enterSelectMode() {
  selectMode.value = true
  selected.value = new Set()
}

function exitSelectMode() {
  selectMode.value = false
  selected.value = new Set()
}

function toggleSelect(uuid: string) {
  const next = new Set(selected.value)
  if (next.has(uuid)) next.delete(uuid)
  else next.add(uuid)
  selected.value = next
}

function toggleSelectAll() {
  if (allSelected.value) {
    selected.value = new Set()
    return
  }
  selected.value = new Set(filtered.value.map((r) => r.uuid))
}

// --- Deletions (real file deletions, always confirmed) ---------------------------

type PendingDelete =
  | { kind: "card"; record: OutputRecord }
  | { kind: "file"; record: OutputRecord; file: OutputFile }
  | { kind: "batch" }

const pendingDelete = ref<PendingDelete | null>(null)
const deleting = ref(false)

const confirmMeta = computed(() => {
  const pending = pendingDelete.value
  if (!pending) return { title: "", message: "" }
  switch (pending.kind) {
    case "card":
      return {
        title: "删除产出卡片",
        message: `确定删除「${pending.record.projectName}」的产出卡片吗？其 ${pending.record.files.length} 个产出文件将被真删除，无法恢复。`,
      }
    case "file":
      return {
        title: "删除产出文件",
        message: `确定真删除文件「${pending.file.name}」吗？无法恢复。`,
      }
    case "batch":
      return {
        title: "批量删除产出卡片",
        message: `确定删除所选 ${selectedCount.value} 个产出卡片吗？对应产出文件将被真删除，无法恢复。`,
      }
  }
})

async function confirmDelete() {
  const pending = pendingDelete.value
  if (!pending || deleting.value) return
  deleting.value = true
  try {
    if (pending.kind === "file") {
      records.value = await removeOutputFile(pending.record.uuid, pending.file.path)
      showToast(`已删除文件「${pending.file.name}」`, "success")
    } else {
      const uuids =
        pending.kind === "card"
          ? [pending.record.uuid]
          : [...selected.value]
      await removeOutputs(uuids)
      showToast(
        pending.kind === "card"
          ? "已删除产出卡片及其文件"
          : `已删除 ${uuids.length} 个产出卡片及其文件`,
        "success"
      )
      if (pending.kind === "batch") exitSelectMode()
      await reload()
    }
    pendingDelete.value = null
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    deleting.value = false
  }
}

// --- Copy artifact file ----------------------------------------------------------

// Copies one artifact to a destination picked through a save dialog.
async function onCopyFile(file: OutputFile) {
  try {
    const { save } = await import("@tauri-apps/plugin-dialog")
    const dest = await save({
      title: "复制产出文件",
      defaultPath: file.name,
    })
    if (!dest) return
    await copyOutputFile(file.path, dest)
    showToast(`已复制到 ${dest}`, "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "复制失败")
  }
}

// --- Loading -----------------------------------------------------------------------

async function reload() {
  records.value = await listOutputs()
  // Reset the template filter when its tag disappeared with the records.
  if (
    templateFilter.value &&
    !templateFilterOptions.value.some((o) => o.value === templateFilter.value)
  ) {
    templateFilter.value = ""
  }
  // Drop selections of records that disappeared.
  if (selectMode.value) {
    const alive = new Set(records.value.map((r) => r.uuid))
    selected.value = new Set([...selected.value].filter((u) => alive.has(u)))
  }
}

onMounted(reload)
// Builds finish while other pages are visible; refresh whenever the
// output page becomes active again.
onActivated(reload)
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      产出区
    </h1>
    <div class="flex shrink-0 items-center gap-3">
      <!-- Search -->
      <div class="relative w-[clamp(140px,16vw,220px)]">
        <Search
          class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
        />
        <input
          ref="searchInput"
          v-model="keyword"
          type="text"
          placeholder="搜索项目、包名、模板或文件名"
          class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input pr-8 pl-8 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
        />
        <button
          v-if="keyword"
          type="button"
          class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1.5 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
          aria-label="清空搜索"
          @click="keyword = ''"
        >
          <X class="size-3" />
        </button>
      </div>
      <!-- Record-date filter -->
      <div class="relative">
        <button
          type="button"
          class="bg-background hover:bg-accent/40 flex h-8 w-[clamp(120px,13vw,170px)] cursor-pointer items-center gap-2 rounded-lg border border-input pr-7 pl-2.5 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
          aria-label="按记录日期筛选"
          title="按产出记录日期筛选"
          @click="openCalendar"
        >
          <Calendar class="text-muted-foreground size-3.5 shrink-0" />
          <span
            class="truncate"
            :class="timeFilter ? 'text-foreground' : 'text-muted-foreground'"
          >
            {{ timeFilter || "按记录日期筛选" }}
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
      <!-- Template tag filter -->
      <div class="flex h-8 items-center gap-1.5">
        <span
          class="text-muted-foreground shrink-0 text-[clamp(10px,1.1vw,11px)]"
        >
          模板
        </span>
        <div class="w-[clamp(110px,12vw,160px)]">
          <AppSelect
            v-model="templateFilter"
            :options="templateFilterOptions"
            aria-label="按模板标签筛选"
          />
        </div>
      </div>
      <!-- Right cluster: mode-dependent actions -->
      <div class="ml-auto flex shrink-0 items-center gap-3">
        <template v-if="selectMode">
          <p
            class="text-muted-foreground shrink-0 truncate text-[clamp(10px,1.1vw,11px)]"
          >
            已选 {{ selectedCount }} 项
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
            @click="pendingDelete = { kind: 'batch' }"
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
        <template v-else>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="!records.length"
            @click="enterSelectMode"
          >
            <Trash2 class="size-3.5" />
            批量删除
          </button>
        </template>
      </div>
    </div>
    <!-- Output cards -->
    <section
      class="flex min-h-0 flex-1 flex-col rounded-xl border border-border"
    >
      <header class="flex shrink-0 items-center justify-between px-3 py-2">
        <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">产出列表</h2>
        <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
          <template v-if="keyword || timeFilter || templateFilter">
            匹配 {{ filtered.length }} / {{ records.length }}
          </template>
          <template v-else>共 {{ records.length }} 个</template>
        </span>
      </header>
      <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
        <template v-if="filtered.length">
          <OutputCard
            v-for="record in filtered"
            :key="record.uuid"
            :record="record"
            :select-mode="selectMode"
            :selected="selected.has(record.uuid)"
            @delete="pendingDelete = { kind: 'card', record }"
            @delete-file="
              (file) => (pendingDelete = { kind: 'file', record, file })
            "
            @copy-file="onCopyFile"
            @toggle-select="toggleSelect(record.uuid)"
          />
        </template>
        <div v-else class="flex flex-1 items-center justify-center">
          <p
            class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]"
          >
            {{
              records.length
                ? "未匹配到产出记录"
                : "暂无产出，构建成功后会自动记录到这里"
            }}
          </p>
        </div>
      </div>
    </section>
    <ConfirmDialog
      v-if="pendingDelete"
      :title="confirmMeta.title"
      :message="confirmMeta.message"
      :busy="deleting"
      @cancel="pendingDelete = null"
      @confirm="confirmDelete"
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
