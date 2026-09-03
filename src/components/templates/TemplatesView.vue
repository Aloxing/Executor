<script setup lang="ts">
import { CheckSquare, Plus, Search, Trash2, X } from "lucide-vue-next"
import { computed, onActivated, onMounted, ref } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import { byCreatedAt } from "@/lib/queues"
import JsonEditorModal from "../JsonEditorModal.vue"
import CreateTemplateModal from "./CreateTemplateModal.vue"
import DeleteConfirmDialog from "./DeleteConfirmDialog.vue"
import TemplateCard from "./TemplateCard.vue"
import TemplateContextMenu from "./TemplateContextMenu.vue"
import {
  deleteTemplates,
  getTemplateDir,
  importCodeTemplate,
  importParameterTemplate,
  listTemplates,
  openInExplorer,
  readParameterJson,
  writeParameterJson,
  type TemplateInfo,
} from "@/lib/templates"
import { showToast } from "@/lib/toast"

const templates = ref<TemplateInfo[]>([])
const showCreate = ref(false)
const editing = ref<TemplateInfo | null>(null)
const keyword = ref("")
// Search input focused through the central search shortcut.
const searchInput = ref<HTMLInputElement | null>(null)

// Page-level shortcuts: primary create button and search focus.
useShortcut("create", () => {
  showCreate.value = true
})
useShortcut("search", () => {
  searchInput.value?.focus()
})

// Batch selection state.
const selectMode = ref(false)
const selected = ref<Set<string>>(new Set())
// Names waiting for delete confirmation (single or batch).
const pendingDelete = ref<string[] | null>(null)
const deleting = ref(false)
// Right-click context menu state.
const menu = ref<{ x: number; y: number; template: TemplateInfo } | null>(null)
// Parameter JSON editor state.
const paramEditing = ref<TemplateInfo | null>(null)
const paramEditorContent = ref("")

// Case-insensitive filter across name, type and description.
const filtered = computed(() => {
  const query = keyword.value.trim().toLowerCase()
  // Newest-first: 降序，最新的排第一。
  const list = !query
    ? templates.value
    : templates.value.filter((t) =>
        [t.name, t.templateType, t.description].some((field) =>
          field.toLowerCase().includes(query)
        )
      )
  return [...list].sort(byCreatedAt)
})

const allFilteredSelected = computed(
  () =>
    filtered.value.length > 0 &&
    filtered.value.every((t) => selected.value.has(t.name))
)

async function reload() {
  templates.value = await listTemplates()
}

onMounted(reload)
// KeepAlive 缓存页面：每次切回时重新加载，保持与磁盘数据同步。
onActivated(reload)

function onSaved(info: TemplateInfo) {
  if (editing.value) {
    const oldName = editing.value.name
    const index = templates.value.findIndex((t) => t.name === oldName)
    if (index >= 0) templates.value[index] = info
    // Keep the selection set consistent when the name changes.
    if (selected.value.has(oldName) && oldName !== info.name) {
      const next = new Set(selected.value)
      next.delete(oldName)
      next.add(info.name)
      selected.value = next
    }
    editing.value = null
  } else {
    templates.value.push(info)
  }
}

function toggleSelect(name: string) {
  const next = new Set(selected.value)
  if (next.has(name)) next.delete(name)
  else next.add(name)
  selected.value = next
}

function toggleSelectAll() {
  const next = new Set(selected.value)
  if (allFilteredSelected.value) {
    for (const t of filtered.value) next.delete(t.name)
  } else {
    for (const t of filtered.value) next.add(t.name)
  }
  selected.value = next
}

function enterSelectMode() {
  selectMode.value = true
  selected.value = new Set()
}

function exitSelectMode() {
  selectMode.value = false
  selected.value = new Set()
}

async function confirmDelete() {
  if (!pendingDelete.value || deleting.value) return
  deleting.value = true
  const targets = pendingDelete.value
  try {
    await deleteTemplates(targets)
    removeLocally(targets)
    pendingDelete.value = null
  } catch (e) {
    // The JSON records are removed before folder cleanup, so refresh the
    // list even when only the folder deletion failed.
    removeLocally(targets)
    showToast(typeof e === "string" ? e : "删除失败，请重试")
    pendingDelete.value = null
  } finally {
    deleting.value = false
  }
}

function removeLocally(names: string[]) {
  const removed = new Set(names)
  templates.value = templates.value.filter((t) => !removed.has(t.name))
  const next = new Set(selected.value)
  for (const name of removed) next.delete(name)
  selected.value = next
}

function openMenu(template: TemplateInfo, event: MouseEvent) {
  menu.value = { x: event.clientX, y: event.clientY, template }
}

async function openInExplorerMenu() {
  if (!menu.value) return
  const template = menu.value.template
  menu.value = null
  try {
    const dir = await getTemplateDir(template.name)
    await openInExplorer(dir)
  } catch (e) {
    showToast(typeof e === "string" ? e : "无法打开文件夹")
  }
}

// Code import: pick a folder, its whole content replaces code/ contents.
async function importCode(template: TemplateInfo) {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择代码模板文件夹",
    })
    if (!selected) return
    await importCodeTemplate(template.name, selected as string)
    markImported(template.name, "code")
    showToast("代码模板导入成功", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "导入失败，请重试")
  }
}

// Parameter import: pick a single JSON file, renamed to <name>.json.
// When the parameter template already exists, open the in-app JSON editor
// instead of the file picker.
async function importParameter(template: TemplateInfo) {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      multiple: false,
      title: "选择参数模板 JSON 文件",
      filters: [{ name: "JSON 文件", extensions: ["json"] }],
    })
    if (!selected) return
    await importParameterTemplate(template.name, selected as string)
    markImported(template.name, "parameter")
    showToast("参数模板导入成功", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "导入失败，请重试")
  }
}

async function openParameterEditor(template: TemplateInfo) {
  try {
    const content = await readParameterJson(template.name)
    paramEditing.value = template
    paramEditorContent.value = content
  } catch (e) {
    showToast(typeof e === "string" ? e : "读取参数模板失败")
  }
}

async function saveParameterJson(content: string) {
  if (!paramEditing.value) return
  try {
    await writeParameterJson(paramEditing.value.name, content)
    showToast("参数模板保存成功", "success")
    paramEditing.value = null
  } catch (e) {
    showToast(typeof e === "string" ? e : "保存失败，请重试")
  }
}

function markImported(name: string, kind: "code" | "parameter") {
  templates.value = templates.value.map((t) =>
    t.name === name
      ? {
          ...t,
          codeImported: kind === "code" ? true : t.codeImported,
          parameterImported: kind === "parameter" ? true : t.parameterImported,
        }
      : t
  )
}

function importCodeFromMenu() {
  if (!menu.value) return
  const template = menu.value.template
  menu.value = null
  importCode(template)
}

function importParameterFromMenu() {
  if (!menu.value) return
  const template = menu.value.template
  menu.value = null
  if (template.parameterImported) {
    // Already imported: open the JSON file in the in-app editor.
    openParameterEditor(template)
    return
  }
  importParameter(template)
}
</script>

<template>
  <div class="flex h-full flex-col gap-3">
    <!-- Page title on its own row, aligned left -->
    <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
      模板
    </h1>
    <div class="flex shrink-0 items-center gap-3">
      <!-- Search box -->
      <div class="relative w-[clamp(180px,24vw,280px)]">
        <Search
          class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
        />
        <input
          ref="searchInput"
          v-model="keyword"
          type="text"
          placeholder="搜索模板名称、类型或介绍"
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
      <p
        class="text-muted-foreground min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)]"
      >
        <template v-if="selectMode">已选 {{ selected.size }} 项</template>
        <template v-else-if="keyword.trim()">
          匹配 {{ filtered.length }} / {{ templates.length }} 个模板
        </template>
        <template v-else>共 {{ templates.length }} 个模板类型</template>
      </p>
      <!-- Batch selection actions -->
      <template v-if="selectMode">
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          :disabled="!filtered.length"
          @click="toggleSelectAll"
        >
          <CheckSquare class="size-3.5" />
          {{ allFilteredSelected ? "取消全选" : "全选" }}
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-destructive/5 px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="!selected.size"
          @click="pendingDelete = [...selected]"
        >
          <Trash2 class="size-3.5" />
          删除所选{{ selected.size ? `(${selected.size})` : "" }}
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
          :disabled="!templates.length"
          title="进入选择模式，批量删除模板"
          @click="enterSelectMode"
        >
          <Trash2 class="size-3.5" />
          批量删除模板
        </button>
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,12px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="showCreate = true"
        >
          <Plus class="size-3.5" />
          创建模板类型
        </button>
      </template>
    </div>
    <div
      v-if="filtered.length"
      class="flex min-h-0 flex-col gap-2 overflow-auto"
    >
      <TemplateCard
        v-for="template in filtered"
        :key="template.name"
        :template="template"
        :select-mode="selectMode"
        :selected="selected.has(template.name)"
        @edit="editing = template"
        @delete="pendingDelete = [template.name]"
        @toggle-select="toggleSelect(template.name)"
        @contextmenu="openMenu(template, $event)"
      />
    </div>
    <div v-else class="flex flex-1 items-center justify-center">
      <p class="text-muted-foreground text-xs">
        {{
          keyword.trim()
            ? "未找到匹配的模板，请调整搜索关键词"
            : "暂无模板，点击右上角「创建模板类型」开始"
        }}
      </p>
    </div>
    <CreateTemplateModal
      v-if="showCreate"
      @close="showCreate = false"
      @saved="onSaved"
    />
    <CreateTemplateModal
      v-if="editing"
      :initial="editing"
      @close="editing = null"
      @saved="onSaved"
    />
    <DeleteConfirmDialog
      v-if="pendingDelete"
      :names="pendingDelete"
      @cancel="pendingDelete = null"
      @confirm="confirmDelete"
    />
    <TemplateContextMenu
      v-if="menu"
      :x="menu.x"
      :y="menu.y"
      :template="menu.template"
      @close="menu = null"
      @open-dir="openInExplorerMenu"
      @import-code="importCodeFromMenu"
      @import-parameter="importParameterFromMenu"
    />
    <JsonEditorModal
      v-if="paramEditing"
      :title="`修改参数模板 · ${paramEditing.name}`"
      :model-value="paramEditorContent"
      @close="paramEditing = null"
      @save="saveParameterJson"
    />
  </div>
</template>
