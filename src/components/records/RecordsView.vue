<script setup lang="ts">
import { Calendar, CheckSquare, Search, Trash2, X } from "lucide-vue-next"
import { computed, onActivated, onMounted, ref } from "vue"
import AppSelect from "../AppSelect.vue"
import CalendarPicker from "../import/CalendarPicker.vue"
import ConfirmDialog from "../import/ConfirmDialog.vue"
import RecordCard from "./RecordCard.vue"
import {
  listRecords,
  removeRecordItem,
  removeRecords,
  type OpRecord,
} from "@/lib/records"
import { useShortcut } from "@/lib/shortcuts"
import { showToast } from "@/lib/toast"

const records = ref<OpRecord[]>([])

// Search: matches the title, detail and sub-record entries.
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

// Operation tag filter (新增/删除/修改).
const actionFilter = ref("")
const actionOptions = [
  { value: "", label: "全部标签" },
  { value: "add", label: "新增" },
  { value: "delete", label: "删除" },
  { value: "modify", label: "修改" },
]

// Page type filter (导入/配置/构建/产出).
const pageFilter = ref("")
const pageOptions = [
  { value: "", label: "全部页面" },
  { value: "import", label: "导入区" },
  { value: "config", label: "配置区" },
  { value: "build", label: "构建区" },
  { value: "output", label: "产出区" },
]

const filtered = computed(() => {
  let list = records.value
  const kw = keyword.value.trim().toLowerCase()
  if (kw) {
    list = list.filter(
      (r) =>
        r.title.toLowerCase().includes(kw) ||
        r.detail.toLowerCase().includes(kw) ||
        r.items.some((item) => item.toLowerCase().includes(kw))
    )
  }
  if (timeFilter.value) {
    list = list.filter((r) => r.createdAt.startsWith(timeFilter.value))
  }
  if (actionFilter.value) {
    list = list.filter((r) => r.action === actionFilter.value)
  }
  if (pageFilter.value) {
    list = list.filter((r) => r.page === pageFilter.value)
  }
  return list
})

const filterActive = computed(
  () => !!(keyword.value || timeFilter.value || actionFilter.value || pageFilter.value)
)

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

// --- Deletions (history entries only; real files are never touched) ---------------

const pendingDelete = ref<{ kind: "card"; record: OpRecord } | { kind: "batch" } | null>(null)
const deleting = ref(false)

const confirmMeta = computed(() => {
  const pending = pendingDelete.value
  if (!pending) return { title: "", message: "" }
  return pending.kind === "card"
    ? {
        title: "删除记录",
        message: `确定删除记录「${pending.record.title}」吗？仅删除历史记录，不影响任何文件。`,
      }
    : {
        title: "批量删除记录",
        message: `确定删除所选 ${selectedCount.value} 条记录吗？仅删除历史记录，不影响任何文件。`,
      }
})

async function confirmDelete() {
  const pending = pendingDelete.value
  if (!pending || deleting.value) return
  deleting.value = true
  try {
    const uuids = pending.kind === "card" ? [pending.record.uuid] : [...selected.value]
    await removeRecords(uuids)
    showToast(
      pending.kind === "card"
        ? "已删除记录"
        : `已删除 ${uuids.length} 条记录`,
      "success"
    )
    if (pending.kind === "batch") exitSelectMode()
    pendingDelete.value = null
    await reload()
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除失败，请重试")
  } finally {
    deleting.value = false
  }
}

// Sub records are plain history entries: deleted directly, no confirm.
async function onDeleteItem(record: OpRecord, index: number) {
  try {
    records.value = await removeRecordItem(record.uuid, index)
    showToast("已删除子记录", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "删除子记录失败")
  }
}

// --- Loading -----------------------------------------------------------------------

async function reload() {
  records.value = await listRecords()
  // Drop selections of records that disappeared.
  if (selectMode.value) {
    const alive = new Set(records.value.map((r) => r.uuid))
    selected.value = new Set([...selected.value].filter((u) => alive.has(u)))
  }
}

onMounted(reload)
// Operations happen on the other pages; refresh whenever the records
// page becomes active again.
onActivated(reload)
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      记录
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
          placeholder="搜索记录标题、详情或子记录"
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
      <!-- Date filter -->
      <div class="relative">
        <button
          type="button"
          class="bg-background hover:bg-accent/40 flex h-8 w-[clamp(120px,13vw,170px)] cursor-pointer items-center gap-2 rounded-lg border border-input pr-7 pl-2.5 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
          aria-label="按记录日期筛选"
          title="按记录日期筛选"
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
      <!-- Operation tag filter -->
      <div class="flex h-8 items-center gap-1.5">
        <span class="text-muted-foreground shrink-0 text-[clamp(10px,1.1vw,11px)]">
          标签
        </span>
        <div class="w-[clamp(120px,13vw,170px)]">
          <AppSelect
            v-model="actionFilter"
            :options="actionOptions"
            aria-label="按操作标签筛选"
          />
        </div>
      </div>
      <!-- Page type filter -->
      <div class="flex h-8 items-center gap-1.5">
        <span class="text-muted-foreground shrink-0 text-[clamp(10px,1.1vw,11px)]">
          页面
        </span>
        <div class="w-[clamp(120px,13vw,170px)]">
          <AppSelect
            v-model="pageFilter"
            :options="pageOptions"
            aria-label="按页面类型筛选"
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
            批量删除记录
          </button>
        </template>
      </div>
    </div>
    <!-- Record cards -->
    <section class="flex min-h-0 flex-1 flex-col rounded-xl border border-border">
      <header class="flex shrink-0 items-center justify-between px-3 py-2">
        <h2 class="text-[clamp(11px,1.3vw,13px)] font-semibold">操作记录</h2>
        <span class="text-muted-foreground text-[clamp(9px,1vw,10px)]">
          <template v-if="filterActive">
            匹配 {{ filtered.length }} / {{ records.length }}
          </template>
          <template v-else>共 {{ records.length }} 条</template>
        </span>
      </header>
      <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
        <template v-if="filtered.length">
          <RecordCard
            v-for="record in filtered"
            :key="record.uuid"
            :record="record"
            :select-mode="selectMode"
            :selected="selected.has(record.uuid)"
            @delete="pendingDelete = { kind: 'card', record }"
            @delete-item="(index) => onDeleteItem(record, index)"
            @toggle-select="toggleSelect(record.uuid)"
          />
        </template>
        <div v-else class="flex flex-1 items-center justify-center">
          <p
            class="text-muted-foreground text-center text-[clamp(10px,1.1vw,11px)]"
          >
            {{
              records.length
                ? "未匹配到记录"
                : "暂无操作记录，各页面的增删改会自动记录到这里"
            }}
          </p>
        </div>
      </div>
    </section>
    <ConfirmDialog
      v-if="pendingDelete"
      :title="confirmMeta.title"
      :message="confirmMeta.message"
      confirm-label="删除"
      danger
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
