import { onActivated, onDeactivated, onMounted, onUnmounted } from "vue"
import { saveSettings, settings } from "./settings"

/**
 * Central shortcut system. Every keyboard shortcut in the app lives here:
 * components register action handlers (LIFO, so the topmost overlay wins)
 * and the single global dispatcher installed by App.vue maps key combos to
 * actions using the user's customization (settings.shortcuts) with these
 * defaults as the fallback.
 */

export const DEFAULT_SHORTCUTS: Record<string, string> = {
  save: "ctrl+s",
  close: "escape",
  settings: "ctrl+,",
  search: "ctrl+f",
  create: "ctrl+n",
  "nav.import": "ctrl+1",
  "nav.config": "ctrl+2",
  "nav.build": "ctrl+3",
  "nav.output": "ctrl+4",
  "nav.records": "ctrl+5",
  "nav.templates": "ctrl+6",
}

export const SHORTCUT_LABELS: Record<string, string> = {
  save: "保存 / 创建 / 确定",
  close: "关闭最上层弹层",
  settings: "打开设置",
  search: "聚焦当前页搜索框",
  create: "当前页主创建按钮",
  "nav.import": "切换到导入区",
  "nav.config": "切换到配置区",
  "nav.build": "切换到构建区",
  "nav.output": "切换到产出区",
  "nav.records": "切换到记录",
  "nav.templates": "切换到模板",
}

/** Display order of the shortcut list in the settings page. */
export const SHORTCUT_ORDER: string[] = [
  "save",
  "close",
  "settings",
  "search",
  "create",
  "nav.import",
  "nav.config",
  "nav.build",
  "nav.output",
  "nav.records",
  "nav.templates",
]

/** A handler returns `false` when it declined (dispatcher keeps looking);
 * async handlers are treated as handled. */
export type ShortcutHandler = () => boolean | void | Promise<unknown>

interface Entry {
  action: string
  handler: ShortcutHandler
}

const registry: Entry[] = []

export function registerShortcut(
  action: string,
  handler: ShortcutHandler
): () => void {
  const entry: Entry = { action, handler }
  registry.push(entry)
  return () => {
    const index = registry.indexOf(entry)
    if (index >= 0) registry.splice(index, 1)
  }
}

/**
 * Registers a shortcut handler for the component's lifetime; also pauses
 * and resumes with KeepAlive activation so cached pages never react.
 */
export function useShortcut(action: string, handler: ShortcutHandler) {
  let off: (() => void) | null = null
  const start = () => {
    if (!off) off = registerShortcut(action, handler)
  }
  const stop = () => {
    off?.()
    off = null
  }
  onMounted(start)
  onActivated(start)
  onDeactivated(stop)
  onUnmounted(stop)
}

/** Current combo of an action: user customization first, default second. */
export function shortcutFor(action: string): string {
  const custom = settings.shortcuts[action]
  if (custom !== undefined) return custom
  return DEFAULT_SHORTCUTS[action] ?? ""
}

/** Action bound to a combo; empty string when nothing is bound. */
export function actionForCombo(combo: string): string {
  if (!combo) return ""
  for (const action of SHORTCUT_ORDER) {
    if (shortcutFor(action) === combo) return action
  }
  return ""
}

/** Renders a stored combo like `ctrl+shift+s` as `Ctrl+Shift+S`. */
export function formatCombo(combo: string): string {
  if (!combo) return "未绑定"
  return combo
    .split("+")
    .map((part) => {
      switch (part) {
        case "ctrl":
          return "Ctrl"
        case "alt":
          return "Alt"
        case "shift":
          return "Shift"
        case "escape":
          return "Esc"
        case " ":
          return "Space"
        default:
          return part.length === 1 ? part.toUpperCase() : part
      }
    })
    .join("+")
}

/** Normalizes a keydown event into the stored combo format. */
export function normalizeCombo(event: KeyboardEvent): string {
  const key = event.key.toLowerCase()
  // Modifier-only presses are not combos.
  if (["control", "shift", "alt", "meta"].includes(key)) return ""
  const parts: string[] = []
  if (event.ctrlKey) parts.push("ctrl")
  if (event.altKey) parts.push("alt")
  if (event.shiftKey) parts.push("shift")
  parts.push(key === " " ? "space" : key)
  return parts.join("+")
}

/**
 * Installs the single global keydown dispatcher (App.vue). Bound combos
 * always preventDefault so the WebView never runs its own action; the
 * topmost (last registered) willing handler of the action then runs.
 */
export function installShortcutDispatcher(): () => void {
  const onKeydown = (event: KeyboardEvent) => {
    const combo = normalizeCombo(event)
    if (!combo) return
    const action = actionForCombo(combo)
    if (!action) return
    event.preventDefault()
    for (let i = registry.length - 1; i >= 0; i--) {
      const entry = registry[i]
      if (entry.action !== action) continue
      if (entry.handler() !== false) return
    }
  }
  window.addEventListener("keydown", onKeydown)
  return () => window.removeEventListener("keydown", onKeydown)
}

/**
 * Binds an action to a new combo. Returns an error message when the combo
 * conflicts with another action; an empty combo unbinds the action.
 */
export function setShortcut(action: string, combo: string): string {
  if (combo) {
    for (const other of SHORTCUT_ORDER) {
      if (other !== action && shortcutFor(other) === combo) {
        return `快捷键 ${formatCombo(combo)} 已被「${SHORTCUT_LABELS[other] ?? other}」使用，请换一个`
      }
    }
  }
  settings.shortcuts = { ...settings.shortcuts, [action]: combo }
  saveSettings()
  return ""
}

/** Restores one action (or all of them) to the default combos. */
export function resetShortcut(action: string) {
  const next = { ...settings.shortcuts }
  delete next[action]
  settings.shortcuts = next
  saveSettings()
}

export function resetAllShortcuts() {
  settings.shortcuts = {}
  saveSettings()
}
