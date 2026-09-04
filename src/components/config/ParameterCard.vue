<script setup lang="ts">
import {
  Calendar,
  CircleCheck,
  CodeXml,
  FileUp,
  ListTree,
  Loader2,
  Plus,
  Save,
  Trash2,
  TriangleAlert,
  X,
} from "lucide-vue-next"
import { computed, onMounted, ref, watch } from "vue"
import AppSelect from "../AppSelect.vue"
import CalendarPicker from "../import/CalendarPicker.vue"
import SettingSwitch from "../settings/SettingSwitch.vue"
import SceneCodeEditor from "./SceneCodeEditor.vue"
import { useShortcut } from "@/lib/shortcuts"
import {
  readProjectParameter,
  writeProjectParameter,
  type ConfigProject,
} from "@/lib/config"
import {
  applyScenes,
  argsText,
  effectiveLogTag,
  generateScenesCode,
  parseArgs,
  parseScenesCode,
  unsupportedStatements,
} from "@/lib/sceneCode"
import { showToast } from "@/lib/toast"

const props = defineProps<{
  project: ConfigProject
  /** Bumped by the card's refresh action to reload from disk (unsaved
   * edits are discarded — that is what an explicit refresh means). */
  refreshTick?: number
}>()

type ParameterKind = "date" | "choice" | "boolean" | "integer" | "number" | "path" | "string"

interface EntryView {
  name: string
  keyName: string
  kind: ParameterKind
  valueFormat: string
  valuePrefix: string
  valueOverride: string
  valueChoices: string[]
}

// The whole parameter document; only `write_mode == "argument"` entries
// get UI controls, everything else is preserved untouched on save so the
// code kernel keeps working.
const doc = ref<Record<string, any> | null>(null)
const original = ref("")
const loading = ref(true)
const error = ref("")
const saving = ref(false)
// Themed calendar popover state, anchored to the trigger button.
const calendar = ref<{ x: number; y: number; name: string } | null>(null)

onMounted(load)

// Card refresh button: re-read the parameter file from the local disk.
watch(
  () => props.refreshTick,
  () => {
    load()
  }
)

async function load() {
  loading.value = true
  error.value = ""
  try {
    const content = await readProjectParameter(props.project.uuid)
    const parsed = JSON.parse(content)
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("参数文件结构不正确")
    }
    doc.value = parsed
    normalizeCodeEntries()
    original.value = JSON.stringify(doc.value)
    initDateDefaults()
    editingArgs.value = null
    // 函数组合区默认就是代码模式：文档重载后按最新 JSON 重新生成草稿。
    enterDefaultCodeMode()
  } catch (e) {
    error.value =
      typeof e === "string" ? e : e instanceof Error ? e.message : "读取参数失败"
  } finally {
    loading.value = false
  }
}

// Entries without a value are initialized with the current time formatted
// by their value_format, so the picker always starts from a valid value.
function initDateDefaults() {
  if (!doc.value) return
  for (const body of Object.values(doc.value)) {
    if (!isArgument(body)) continue
    const format = stringValue(body.value_format)
    if (!format) continue
    if (typeof body.value !== "string" || !body.value.trim()) {
      body.value = formatPattern(new Date(), format)
    }
  }
}

function isArgument(body: any): boolean {
  return !!body && typeof body === "object" && body.write_mode === "argument"
}

// Ensure every body statement of every code scene has the full structure
// (type / call / rule), so the dropdowns can v-model straight into the
// JSON without hitting undefined branches.
function normalizeCodeEntries() {
  if (!doc.value) return
  for (const name of Object.keys(doc.value)) {
    const entry = doc.value[name]
    if (!entry || typeof entry !== "object" || entry.write_mode !== "code") continue
    const scenes =
      entry.scenes && typeof entry.scenes === "object" && !Array.isArray(entry.scenes)
        ? entry.scenes
        : {}
    for (const scene of Object.values(scenes) as any[]) {
      if (!scene || typeof scene !== "object" || !Array.isArray(scene.body)) continue
      for (const stmt of scene.body) {
        if (!stmt || typeof stmt !== "object") continue
        if (typeof stmt.type !== "string") stmt.type = "direct"
        if (!stmt.call || typeof stmt.call !== "object") {
          stmt.call = { callback: "", args: [] }
        }
        if (stmt.type === "ruled" && (!stmt.rule || typeof stmt.rule !== "object")) {
          stmt.rule = { template: "", args: [] }
        }
      }
    }
  }
}

const statementTypeOptions = [
  { value: "direct", label: "direct 直调" },
  { value: "ruled", label: "ruled 规则" },
]

function optionList(names: string[]): { value: string; label: string }[] {
  return names.map((n) => ({ value: n, label: n }))
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : ""
}

function detectKind(body: any): ParameterKind {
  const choices = Array.isArray(body?.value_choice) ? body.value_choice : []
  if (choices.length > 0) return "choice"
  if (stringValue(body.value_format)) return "date"
  if (body.value === null) return "integer"
  if (typeof body.value === "boolean") return "boolean"
  if (typeof body.value === "number") return "number"
  // file_path being a directory (no extension, empty file_type) means the
  // value is the source file address to copy from.
  const filePath = stringValue(body.file_path)
  if (!stringValue(body.file_type) && filePath && !/\.[A-Za-z0-9]+$/.test(filePath)) {
    return "path"
  }
  return "string"
}

const entries = computed<EntryView[]>(() => {
  if (!doc.value) return []
  return Object.entries(doc.value)
    .filter(([, body]) => isArgument(body))
    .map(([name, body]) => ({
      name,
      keyName: stringValue(body.key_name) || name,
      kind: detectKind(body),
      valueFormat: stringValue(body.value_format),
      valuePrefix: stringValue(body.value_prefix),
      valueOverride: stringValue(body.value_override),
      valueChoices: Array.isArray(body.value_choice)
        ? body.value_choice.map((c: unknown) =>
            typeof c === "string" ? c : String(c)
          )
        : [],
    }))
})

// --- code entries: function composition ---------------------------------
// A `write_mode == "code"` entry holds `scenes` (generated Java methods),
// whose bodies assemble statements from `ruleTemplates` (rule components)
// and `callbackTemplates` (callback components).

const codeEntries = computed<string[]>(() => {
  if (!doc.value) return []
  return Object.entries(doc.value)
    .filter(
      ([, body]) => !!body && typeof body === "object" && body.write_mode === "code"
    )
    .map(([name]) => name)
})

function scenesOf(codeName: string): Record<string, any> {
  const entry = doc.value?.[codeName]
  if (!entry || typeof entry !== "object") return {}
  if (!entry.scenes || typeof entry.scenes !== "object" || Array.isArray(entry.scenes)) {
    entry.scenes = {}
  }
  return entry.scenes
}

function ruleOptions(codeName: string): string[] {
  const templates = doc.value?.[codeName]?.ruleTemplates
  return templates && typeof templates === "object" ? Object.keys(templates) : []
}

function callbackOptions(codeName: string): string[] {
  const templates = doc.value?.[codeName]?.callbackTemplates
  return templates && typeof templates === "object" ? Object.keys(templates) : []
}

function sceneBodyList(codeName: string, sceneName: string): any[] {
  const scene = scenesOf(codeName)[sceneName]
  if (!scene || typeof scene !== "object") return []
  if (!Array.isArray(scene.body)) scene.body = []
  return scene.body
}

function addScene(codeName: string) {
  const scenes = scenesOf(codeName)
  for (let i = 1; ; i++) {
    const name = `Function${i}`
    if (!(name in scenes)) {
      // New functions default to a void return type.
      scenes[name] = { returnType: "void", body: [] }
      return
    }
  }
}

function removeScene(codeName: string, sceneName: string) {
  delete scenesOf(codeName)[sceneName]
}

// Renaming rebuilds the scenes object so the key changes in place; the
// new name must be non-empty and unique within the entry.
function renameScene(codeName: string, oldName: string, newName: string) {
  const entry = doc.value?.[codeName]
  const scenes = entry?.scenes
  if (!entry || !scenes || typeof scenes !== "object") return
  const trimmed = newName.trim()
  if (!trimmed || trimmed === oldName) return
  if (trimmed in scenes) {
    showToast(`函数「${trimmed}」已存在，请换一个名称`)
    return
  }
  const rebuilt: Record<string, any> = {}
  for (const [key, value] of Object.entries(scenes)) {
    rebuilt[key === oldName ? trimmed : key] = value
  }
  entry.scenes = rebuilt
}

function onRenameScene(codeName: string, oldName: string, event: Event) {
  const input = event.target as HTMLInputElement
  renameScene(codeName, oldName, input.value)
  // Rejected rename: sync the input back to the current name.
  if (oldName in scenesOf(codeName)) input.value = oldName
}

// Formal parameters (params: [{type, name}]); rendering never creates the
// array so untouched scenes stay byte-identical until the user adds one.
function sceneParams(codeName: string, sceneName: string): any[] {
  const scene = scenesOf(codeName)[sceneName]
  return scene && Array.isArray(scene.params) ? scene.params : []
}

function addParam(codeName: string, sceneName: string) {
  const scene = scenesOf(codeName)[sceneName]
  if (!scene || typeof scene !== "object") return
  if (!Array.isArray(scene.params)) scene.params = []
  scene.params.push({ type: "", name: "" })
}

function removeParam(codeName: string, sceneName: string, index: number) {
  const scene = scenesOf(codeName)[sceneName]
  if (scene && Array.isArray(scene.params)) scene.params.splice(index, 1)
}

function addStatement(codeName: string, sceneName: string) {
  sceneBodyList(codeName, sceneName).push({
    type: "direct",
    call: { callback: callbackOptions(codeName)[0] ?? "", args: [] },
  })
}

function removeStatement(codeName: string, sceneName: string, index: number) {
  sceneBodyList(codeName, sceneName).splice(index, 1)
}

// Switching to `ruled` ensures a rule object bound to the first rule
// template; the callback call is kept across type changes.
function setStatementType(
  codeName: string,
  sceneName: string,
  index: number,
  type: string
) {
  const stmt = sceneBodyList(codeName, sceneName)[index]
  if (!stmt || typeof stmt !== "object") return
  stmt.type = type
  if (type === "ruled" && (!stmt.rule || typeof stmt.rule !== "object")) {
    stmt.rule = { template: ruleOptions(codeName)[0] ?? "", args: [] }
  }
  if (!stmt.call || typeof stmt.call !== "object") {
    stmt.call = { callback: callbackOptions(codeName)[0] ?? "", args: [] }
  }
}

// Argument tokens (rendering and parsing) live in lib/sceneCode.ts so the
// 实参 inputs and the 代码模式 preview share one serialization.

// ruled statements have rule args; every statement also has callback call
// args (both are comma-separated token lists; numbers stay numbers).
function setRuleArgs(
  codeName: string,
  sceneName: string,
  index: number,
  text: string
) {
  const stmt = sceneBodyList(codeName, sceneName)[index]
  if (!stmt || typeof stmt !== "object") return
  if (!stmt.rule || typeof stmt.rule !== "object") stmt.rule = { template: "", args: [] }
  stmt.rule.args = parseArgs(text)
}

function setCallArgs(
  codeName: string,
  sceneName: string,
  index: number,
  text: string
) {
  const stmt = sceneBodyList(codeName, sceneName)[index]
  if (!stmt || typeof stmt !== "object") return
  if (!stmt.call || typeof stmt.call !== "object") stmt.call = { callback: "", args: [] }
  stmt.call.args = parseArgs(text)
}

// While an args input is focused it shows exactly what the user types
// (commas included); the normalized argsText is only applied on blur, so
// partially-typed states never get overwritten mid-edit.
const editingArgs = ref<{ key: string; text: string } | null>(null)

function argsDisplay(key: string, args: unknown): string {
  if (editingArgs.value && editingArgs.value.key === key) {
    return editingArgs.value.text
  }
  return argsText(args)
}

function onArgsFocus(key: string, event: FocusEvent) {
  editingArgs.value = { key, text: (event.target as HTMLInputElement).value }
}

function onArgsInput(
  key: string,
  codeName: string,
  sceneName: string,
  index: number,
  which: "rule" | "call",
  event: Event
) {
  const text = (event.target as HTMLInputElement).value
  editingArgs.value = { key, text }
  if (which === "rule") setRuleArgs(codeName, sceneName, index, text)
  else setCallArgs(codeName, sceneName, index, text)
}

function onArgsBlur() {
  editingArgs.value = null
}

// --- 代码模式：scenes ⇄ 等价 Java 方法 --------------------------------------

// Per code entry: whether the editor replaces the form, the editor text,
// and the lines that could not be mapped back into the JSON.
const codeMode = ref<Record<string, boolean>>({})
const codeDraft = ref<Record<string, string>>({})
const codeIssues = ref<Record<string, string[]>>({})

function isCodeMode(codeName: string): boolean {
  return !!codeMode.value[codeName]
}

/** Parse issues of one code entry (empty while everything maps back). */
function issuesOf(codeName: string): string[] {
  return codeIssues.value[codeName] ?? []
}

/** Statements without a Java form (the kernel skips them); editing their
 * function in code mode drops them, so the mode warns about them. */
function unsupportedOf(codeName: string): string[] {
  return unsupportedStatements(doc.value?.[codeName])
}

// 代码模式 is the default presentation of the function composition area:
// every code entry starts in the editor, and 组合模式 is the opt-in form.
function enterDefaultCodeMode() {
  codeMode.value = {}
  codeDraft.value = {}
  codeIssues.value = {}
  for (const codeName of codeEntries.value) {
    prepareCodeDraft(codeName)
    codeMode.value[codeName] = true
  }
}

/** Renders the entry's scenes as the exact Java source the code kernel
 * would inject (same signature, same Log.i line, same order). */
function prepareCodeDraft(codeName: string) {
  codeDraft.value[codeName] = generateScenesCode(doc.value?.[codeName])
  codeIssues.value[codeName] = []
}

/** User-facing switch into the mode; also warns about the statements that
 * have no Java form and would be lost by editing their function. */
function enterCodeMode(codeName: string) {
  prepareCodeDraft(codeName)
  codeMode.value[codeName] = true
  const unsupported = unsupportedOf(codeName)
  if (unsupported.length) {
    showToast(
      `${unsupported.length} 条语句类型不受支持（${unsupported[0]}），代码中无法呈现，编辑对应函数会丢失它们`
    )
  }
}

function exitCodeMode(codeName: string) {
  codeMode.value[codeName] = false
  const issues = codeIssues.value[codeName] ?? []
  if (issues.length) {
    showToast(
      `有 ${issues.length} 处代码无法解析，JSON 保持上一次有效内容`,
      "info"
    )
  }
}

// Every keystroke reparses the whole text. The JSON is rewritten only when
// every line maps back to the schema, so a half-typed statement can never
// corrupt the parameter file; the unresolved lines are reported instead.
function onCodeInput(codeName: string, text: string) {
  codeDraft.value[codeName] = text
  syncFromCode(codeName)
}

function syncFromCode(codeName: string) {
  const entry = doc.value?.[codeName]
  if (!entry || typeof entry !== "object") return
  const ruleTemplates =
    entry.ruleTemplates && typeof entry.ruleTemplates === "object"
      ? entry.ruleTemplates
      : {}
  const result = parseScenesCode(
    codeDraft.value[codeName] ?? "",
    ruleTemplates
  )
  codeIssues.value[codeName] = result.errors
  if (result.errors.length) return

  const scenes = entry.scenes
  const existing =
    scenes && typeof scenes === "object" && !Array.isArray(scenes) ? scenes : {}
  const currentTag = effectiveLogTag(entry)
  // The tag of the Log.i lines belongs to the entry, so editing it in code
  // flows back; an absent tag keeps the kernel's default.
  const logTag = result.logTag || currentTag
  entry.scenes = applyScenes(existing, result, ruleTemplates, logTag)
  if (result.logTag && result.logTag !== currentTag) {
    entry.logTag = result.logTag
  }
}

// value_override references another argument entry whose value overrides
// this one; such entries render read-only with the source's live value.
function overrideSource(entry: EntryView): EntryView | null {
  if (!entry.valueOverride) return null
  return entries.value.find((e) => e.name === entry.valueOverride) ?? null
}

const dirty = computed(
  () => !!doc.value && JSON.stringify(doc.value) !== original.value
)

// --- Value setters (kept type-stable for the kernel) -----------------------

function setText(name: string, value: string) {
  const body = doc.value?.[name]
  if (body) body.value = value
}

function setNumber(name: string, raw: string, integer: boolean) {
  const body = doc.value?.[name]
  if (!body) return
  const num = Number(raw)
  body.value = Number.isFinite(num) && raw !== "" ? (integer ? Math.trunc(num) : num) : 0
}

function setBool(name: string, value: boolean) {
  const body = doc.value?.[name]
  if (body) body.value = value
}

// --- Date helpers -----------------------------------------------------------

function formatPattern(date: Date, pattern: string): string {
  const pad = (n: number) => String(n).padStart(2, "0")
  const hours12 = () => {
    const h = date.getHours() % 12
    return h === 0 ? 12 : h
  }
  return pattern
    .replace(/yyyy/g, String(date.getFullYear()))
    .replace(/MM/g, pad(date.getMonth() + 1))
    .replace(/dd/g, pad(date.getDate()))
    .replace(/HH/g, pad(date.getHours()))
    .replace(/hh/g, pad(hours12()))
    .replace(/mm/g, pad(date.getMinutes()))
    .replace(/ss/g, pad(date.getSeconds()))
}

function formatHasTime(pattern: string): boolean {
  return /[Hhms]/.test(pattern.replace(/yyyy|MM|dd/g, ""))
}

// The picker highlights by YYYY-MM-DD; extract that prefix when present.
function dateOnly(value: unknown): string {
  const s = String(value ?? "")
  return /^\d{4}-\d{2}-\d{2}/.test(s) ? s.slice(0, 10) : ""
}

function openCalendar(entry: EntryView, event: MouseEvent) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  calendar.value = { x: rect.left, y: rect.bottom + 4, name: entry.name }
}

// The picker yields a date; the time part comes from now when the format
// includes time components. An empty string means the picker's 清除
// action: fall back to the current time so the value stays valid.
function onPickDate(dateStr: string) {
  const target = calendar.value
  calendar.value = null
  if (!target || !doc.value) return
  const body = doc.value[target.name]
  const format = stringValue(body?.value_format)
  if (!body || !format) return
  if (!dateStr) {
    body.value = formatPattern(new Date(), format)
    return
  }
  const [y, m, d] = dateStr.split("-").map(Number)
  const now = new Date()
  const dt = formatHasTime(format)
    ? new Date(y, m - 1, d, now.getHours(), now.getMinutes(), now.getSeconds())
    : new Date(y, m - 1, d)
  body.value = formatPattern(dt, format)
}

// File address fields: pick a file through the native dialog and use its
// address as the value (the kernel copies it into the directory).
async function pickFile(name: string) {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      multiple: false,
      title: "选择需要复制的文件",
    })
    if (!selected) return
    setText(name, selected as string)
  } catch (e) {
    showToast(typeof e === "string" ? e : "打开文件选择失败")
  }
}

// The calendar popover and saving are driven by the central shortcuts;
// without an open calendar the close event falls through to other layers.
useShortcut("close", () => {
  if (calendar.value) {
    calendar.value = null
    return true
  }
  return false
})
useShortcut("save", () => save())

// --- Actions -----------------------------------------------------------------

async function save() {
  if (!doc.value || saving.value) return
  saving.value = true
  try {
    const content = JSON.stringify(doc.value, null, 4)
    await writeProjectParameter(props.project.uuid, content)
    original.value = JSON.stringify(doc.value)
    showToast("参数已保存", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "保存失败，请重试")
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="flex w-full flex-col gap-2" @click.stop>
    <!-- Loading / error / empty states -->
    <div
      v-if="loading"
      class="text-muted-foreground flex items-center justify-center gap-1.5 py-2 text-[clamp(9px,1vw,10px)]"
    >
      <Loader2 class="size-3 animate-spin" />
      参数加载中…
    </div>
    <p
      v-else-if="error"
      class="text-muted-foreground px-1 py-1 text-center text-[clamp(9px,1vw,10px)]"
    >
      {{ error }}
    </p>
    <!-- key_name: value rows -->
    <template v-else-if="doc">
      <p
        v-if="!entries.length && !codeEntries.length"
        class="text-muted-foreground px-1 py-1 text-center text-[clamp(9px,1vw,10px)]"
      >
        暂无可编辑的参数内容（argument / code）
      </p>
      <div
        v-for="entry in entries"
        :key="entry.name"
        class="flex items-center gap-2"
      >
        <span
          class="text-muted-foreground w-2/5 min-w-0 shrink-0 truncate text-[clamp(9px,1vw,10px)]"
          :title="`${entry.name}（${entry.keyName}）`"
        >
          {{ entry.keyName }}
        </span>
        <!-- Prefix badge: applied by the kernel at injection time -->
        <span
          v-if="entry.valuePrefix"
          class="bg-muted text-muted-foreground shrink-0 rounded px-1 font-mono text-[clamp(8px,0.9vw,9px)]"
          :title="`注入时添加前缀：${entry.valuePrefix}`"
        >
          {{ entry.valuePrefix }}
        </span>
        <!-- Overridden: read-only live copy of the source entry's value -->
        <template v-if="overrideSource(entry)">
          <p
            class="bg-muted/40 text-muted-foreground h-7 min-w-0 flex-1 truncate rounded-md border border-input px-2 leading-7 font-mono text-[clamp(9px,1vw,10px)]"
            :title="`值由「${overrideSource(entry)!.keyName}」覆盖`"
          >
            {{ doc[overrideSource(entry)!.name]?.value ?? "" }}
          </p>
        </template>
        <!-- Date picker -->
        <div
          v-else-if="entry.kind === 'date'"
          class="relative min-w-0 flex-1"
        >
          <input
            :value="String(doc[entry.name]?.value ?? '')"
            type="text"
            readonly
            class="bg-background h-7 w-full cursor-pointer rounded-md border border-input pr-7 pl-2 font-mono text-[clamp(9px,1vw,10px)] focus-visible:outline-none"
            :title="entry.valueFormat"
            @click="openCalendar(entry, $event)"
          />
          <button
            type="button"
            class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded bg-transparent transition-colors focus-visible:outline-none"
            aria-label="选择日期"
            @click.stop="openCalendar(entry, $event)"
          >
            <Calendar class="size-3" />
          </button>
        </div>
        <!-- Choice dropdown (value_choice) -->
        <AppSelect
          v-else-if="entry.kind === 'choice'"
          :model-value="String(doc[entry.name]?.value ?? '')"
          :options="entry.valueChoices.map((c) => ({ value: c, label: c }))"
          :aria-label="entry.keyName"
          class="min-w-0 flex-1"
          @update:model-value="setText(entry.name, $event)"
        />
        <!-- Switch -->
        <div
          v-else-if="entry.kind === 'boolean'"
          class="flex min-w-0 flex-1 items-center"
        >
          <SettingSwitch
            :model-value="doc[entry.name]?.value === true"
            active-class="bg-sky-400"
            @update:model-value="setBool(entry.name, $event)"
          />
        </div>
        <!-- Integer / number -->
        <input
          v-else-if="entry.kind === 'integer' || entry.kind === 'number'"
          :value="String(doc[entry.name]?.value ?? 0)"
          type="number"
          class="bg-background focus-visible:ring-ring h-7 min-w-0 flex-1 rounded-md border border-input px-2 font-mono text-[clamp(9px,1vw,10px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          @input="
            setNumber(entry.name, ($event.target as HTMLInputElement).value, entry.kind === 'integer')
          "
        />
        <!-- Text -->
        <input
          v-else-if="entry.kind !== 'path'"
          :value="String(doc[entry.name]?.value ?? '')"
          type="text"
          class="bg-background focus-visible:ring-ring h-7 min-w-0 flex-1 rounded-md border border-input px-2 font-mono text-[clamp(9px,1vw,10px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          @input="setText(entry.name, ($event.target as HTMLInputElement).value)"
        />
        <!-- File address with a native file picker -->
        <div
          v-else
          class="relative min-w-0 flex-1"
        >
          <input
            :value="String(doc[entry.name]?.value ?? '')"
            type="text"
            class="bg-background focus-visible:ring-ring h-7 w-full rounded-md border border-input pr-7 pl-2 font-mono text-[clamp(9px,1vw,10px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
            placeholder="需要复制的文件地址"
            @input="setText(entry.name, ($event.target as HTMLInputElement).value)"
          />
          <button
            type="button"
            class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded bg-transparent transition-colors focus-visible:outline-none"
            aria-label="选择文件"
            title="选择文件"
            @click.stop="pickFile(entry.name)"
          >
            <FileUp class="size-3" />
          </button>
        </div>
      </div>
      <!-- code entries: function composition -->
      <div
        v-for="codeName in codeEntries"
        :key="codeName"
        class="flex flex-col gap-2 border-t-2 border-border pt-2.5"
      >
        <div class="flex items-center justify-between gap-2">
          <p
            class="min-w-0 truncate text-[clamp(10px,1.1vw,11px)] font-semibold"
            :title="codeName"
          >
            函数组合
          </p>
          <div class="flex shrink-0 items-center gap-1">
            <!-- 代码模式: the same scenes as editable Java methods -->
            <button
              type="button"
              class="hover:bg-muted inline-flex h-6 cursor-pointer items-center gap-1 rounded-md bg-muted/60 px-2 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none"
              :title="
                isCodeMode(codeName)
                  ? '回到函数组合表单（代码已回写到 JSON）'
                  : '生成等价函数代码直接编辑，改动实时回写到 JSON'
              "
              @click="
                isCodeMode(codeName)
                  ? exitCodeMode(codeName)
                  : enterCodeMode(codeName)
              "
            >
              <ListTree v-if="isCodeMode(codeName)" class="size-2.5" />
              <CodeXml v-else class="size-2.5" />
              {{ isCodeMode(codeName) ? "组合模式" : "代码模式" }}
            </button>
            <button
              v-if="!isCodeMode(codeName)"
              type="button"
              class="hover:bg-muted inline-flex h-6 shrink-0 cursor-pointer items-center gap-1 rounded-md bg-muted/60 px-2 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none"
              @click="addScene(codeName)"
            >
              <Plus class="size-2.5" />
              新建函数
            </button>
          </div>
        </div>
        <!-- 代码模式：编辑器 + 回写状态 -->
        <template v-if="isCodeMode(codeName)">
          <div class="h-[clamp(280px,46vh,560px)]">
            <SceneCodeEditor
              :model-value="codeDraft[codeName] ?? ''"
              @update:model-value="onCodeInput(codeName, $event)"
            />
          </div>
          <div
            class="flex items-start gap-1 text-[clamp(8px,0.9vw,9px)]"
            :class="
              issuesOf(codeName).length
                ? 'text-destructive'
                : 'text-muted-foreground'
            "
            role="status"
          >
            <TriangleAlert
              v-if="issuesOf(codeName).length"
              class="mt-px size-2.5 shrink-0"
            />
            <CircleCheck v-else class="mt-px size-2.5 shrink-0" />
            <span
              v-if="issuesOf(codeName).length"
              :title="issuesOf(codeName).join('\n')"
            >
              {{ issuesOf(codeName).length }} 处无法回写到 JSON：{{
                issuesOf(codeName)[0]
              }}{{ issuesOf(codeName).length > 1 ? " 等" : "" }}（JSON 保持上一次有效内容）
            </span>
            <span v-else>已按函数格式回写到 JSON（scenes）</span>
          </div>
          <p
            v-if="unsupportedOf(codeName).length"
            class="text-[clamp(8px,0.9vw,9px)] text-amber-600 dark:text-amber-500"
            :title="unsupportedOf(codeName).join('\n')"
          >
            {{ unsupportedOf(codeName).length }} 条语句类型不受支持（非 direct / ruled），代码中无法呈现，编辑对应函数会丢失它们。
          </p>
          <p
            class="text-muted-foreground text-[clamp(8px,0.9vw,9px)] leading-relaxed"
          >
            语句格式：callback(args);（direct）与 Class.method(ruleArgs, this::callback);（ruled，Class/method 取自 ruleTemplates）；Log.i 行由内核生成；return 表达式对应函数的 return 字段。
          </p>
        </template>
        <template v-else>
        <p
          v-if="!Object.keys(scenesOf(codeName)).length"
          class="text-muted-foreground px-1 py-1 text-center text-[clamp(9px,1vw,10px)]"
        >
          暂无函数，点击「新建函数」创建（可自定义函数名与形参）
        </p>
        <!-- Function cards, each preceded by a numbered title -->
        <div class="flex flex-col gap-3">
        <template
          v-for="(scene, sceneName, sceneIndex) in scenesOf(codeName)"
          :key="sceneName"
        >
          <div class="flex flex-col gap-1.5">
          <div>
            <span
              class="rounded-md bg-sky-500/15 px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium text-sky-600 dark:text-sky-500"
            >
              函数{{ sceneIndex + 1 }}
            </span>
          </div>
        <div
          class="flex flex-col gap-1.5 rounded-md border border-border bg-background/40 p-1.5"
        >
          <!-- Scene header: editable function name + return type + delete -->
          <div class="flex items-center gap-1.5">
            <input
              :value="String(sceneName)"
              type="text"
              placeholder="函数名称"
              class="bg-background focus-visible:ring-ring h-6 min-w-0 flex-1 rounded-md border border-input px-1.5 text-[clamp(9px,1vw,10px)] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2"
              aria-label="函数名称"
              title="修改后回车生效（重命名）"
              @change="onRenameScene(codeName, String(sceneName), $event)"
            />
            <input
              :value="String(scene?.returnType ?? '')"
              type="text"
              placeholder="返回类型（默认 void）"
              class="bg-background focus-visible:ring-ring h-6 w-[120px] shrink-0 rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
              @input="scene && (scene.returnType = ($event.target as HTMLInputElement).value)"
            />
            <button
              type="button"
              class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors duration-200 focus-visible:outline-none"
              aria-label="删除函数"
              title="删除该函数（从 scenes 中移除）"
              @click="removeScene(codeName, String(sceneName))"
            >
              <Trash2 class="size-3" />
            </button>
          </div>
          <!-- Formal parameters -->
          <div class="flex flex-col gap-1">
            <div
              v-for="(param, pIndex) in sceneParams(codeName, String(sceneName))"
              :key="pIndex"
              class="flex items-center gap-1"
            >
              <input
                v-model="param.type"
                type="text"
                placeholder="形参类型（如 String）"
                class="bg-background focus-visible:ring-ring h-6 min-w-0 flex-1 rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
                aria-label="形参类型"
              />
              <input
                v-model="param.name"
                type="text"
                placeholder="形参名称"
                class="bg-background focus-visible:ring-ring h-6 min-w-0 flex-1 rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
                aria-label="形参名称"
              />
              <button
                type="button"
                class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded transition-colors duration-200 focus-visible:outline-none"
                aria-label="删除形参"
                title="删除该形参"
                @click="removeParam(codeName, String(sceneName), pIndex)"
              >
                <X class="size-2.5" />
              </button>
            </div>
            <button
              type="button"
              class="text-muted-foreground hover:bg-muted hover:text-foreground flex h-5 cursor-pointer items-center justify-center gap-1 rounded border border-dashed border-border/60 bg-transparent text-[clamp(8px,0.9vw,9px)] transition-colors duration-200 focus-visible:outline-none"
              @click="addParam(codeName, String(sceneName))"
            >
              <Plus class="size-2" />
              添加形参
            </button>
          </div>
          <!-- Body statements -->
          <div
            v-for="(stmt, index) in sceneBodyList(codeName, String(sceneName))"
            :key="index"
            class="flex flex-col gap-1 rounded border border-border/40 bg-muted/20 p-1"
          >
            <div class="flex flex-wrap items-center gap-1">
              <AppSelect
                v-model="stmt.type"
                :options="statementTypeOptions"
                aria-label="语句类型"
                class="shrink-0"
                @update:model-value="setStatementType(codeName, String(sceneName), index, $event)"
              />
              <AppSelect
                v-if="stmt?.type === 'ruled'"
                v-model="stmt.rule.template"
                :options="optionList(ruleOptions(codeName))"
                aria-label="规则模板"
              />
              <AppSelect
                v-model="stmt.call.callback"
                :options="optionList(callbackOptions(codeName))"
                aria-label="回调"
              />
              <button
                type="button"
                class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded transition-colors duration-200 focus-visible:outline-none"
                aria-label="删除语句"
                title="删除该语句"
                @click="removeStatement(codeName, String(sceneName), index)"
              >
                <X class="size-2.5" />
              </button>
            </div>
            <!-- Rule args (ruled statements only) -->
            <div v-if="stmt?.type === 'ruled'" class="flex items-center gap-1">
              <span
                class="text-muted-foreground w-14 shrink-0 text-right text-[clamp(8px,0.9vw,9px)]"
              >
                规则实参
              </span>
              <input
                :value="argsDisplay(`${codeName}|${sceneName}|${index}|rule`, stmt?.rule?.args)"
                type="text"
                placeholder="&quot;p1&quot;,p2,10"
                class="bg-background focus-visible:ring-ring h-6 min-w-0 flex-1 rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
                aria-label="规则实参"
                @focus="onArgsFocus(`${codeName}|${sceneName}|${index}|rule`, $event)"
                @input="onArgsInput(`${codeName}|${sceneName}|${index}|rule`, codeName, String(sceneName), index, 'rule', $event)"
                @blur="onArgsBlur"
              />
            </div>
            <!-- Callback call args (every statement) -->
            <div class="flex items-center gap-1">
              <span
                class="text-muted-foreground w-14 shrink-0 text-right text-[clamp(8px,0.9vw,9px)]"
              >
                回调实参
              </span>
              <input
                :value="argsDisplay(`${codeName}|${sceneName}|${index}|call`, stmt?.call?.args)"
                type="text"
                placeholder="&quot;p1&quot;,p2,10"
                class="bg-background focus-visible:ring-ring h-6 min-w-0 flex-1 rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
                aria-label="回调实参"
                @focus="onArgsFocus(`${codeName}|${sceneName}|${index}|call`, $event)"
                @input="onArgsInput(`${codeName}|${sceneName}|${index}|call`, codeName, String(sceneName), index, 'call', $event)"
                @blur="onArgsBlur"
              />
            </div>
          </div>
          <button
            type="button"
            class="text-muted-foreground hover:bg-muted hover:text-foreground flex h-6 cursor-pointer items-center justify-center gap-1 rounded-md border border-dashed border-border/60 bg-transparent text-[clamp(8px,0.9vw,9px)] transition-colors duration-200 focus-visible:outline-none"
            @click="addStatement(codeName, String(sceneName))"
          >
            <Plus class="size-2.5" />
            添加语句
          </button>
          <!-- Optional trailing return expression -->
          <input
            :value="String(scene?.return ?? '')"
            type="text"
            placeholder="return 表达式（可选）"
            class="bg-background focus-visible:ring-ring h-6 w-full rounded-md border border-input px-1.5 font-mono text-[clamp(8px,0.9vw,9px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
            @input="scene && (scene.return = ($event.target as HTMLInputElement).value)"
          />
        </div>
          </div>
        </template>
        </div>
        </template>
      </div>
      <!-- Save (template reset moved to the card's context menu) -->
      <div class="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-6 cursor-pointer items-center gap-1 rounded-md px-2 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="saving || !dirty"
          title="保存参数修改（写入 config/parameter/包名.json）"
          @click="save"
        >
          <Loader2 v-if="saving" class="size-2.5 animate-spin" />
          <Save v-else class="size-2.5" />
          保存
        </button>
      </div>
    </template>
    <CalendarPicker
      v-if="calendar"
      :x="calendar.x"
      :y="calendar.y"
      :model-value="dateOnly(doc?.[calendar.name]?.value)"
      @close="calendar = null"
      @pick="onPickDate"
    />
  </div>
</template>
