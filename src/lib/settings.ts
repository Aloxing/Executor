import { reactive } from "vue"

export type CloseBehavior = "ask" | "tray" | "exit"
export type ThemeMode = "dark" | "light" | "system"

/** One Gradle installation directory (the executable lives in `bin`). */
export interface GradleEnv {
  name: string
  path: string
}

export interface AppSettings {
  closeBehavior: CloseBehavior
  workspacePath: string
  themeMode: ThemeMode
  gradleEnvs: GradleEnv[]
  /** Customized shortcut combos per action id; defaults live in
   * `lib/shortcuts.ts`. An empty string unbinds an action. */
  shortcuts: Record<string, string>
}

const defaults: AppSettings = {
  closeBehavior: "ask",
  workspacePath: "",
  themeMode: "system",
  gradleEnvs: [],
  shortcuts: {},
}

export const settings = reactive<AppSettings>({ ...defaults })

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function loadSettings(): Promise<void> {
  try {
    const loaded = await invoke<AppSettings>("load_settings")
    settings.closeBehavior =
      loaded.closeBehavior === "tray" || loaded.closeBehavior === "exit"
        ? loaded.closeBehavior
        : "ask"
    settings.workspacePath = loaded.workspacePath ?? ""
    settings.themeMode =
      loaded.themeMode === "dark" || loaded.themeMode === "light"
        ? loaded.themeMode
        : "system"
    settings.gradleEnvs = Array.isArray(loaded.gradleEnvs)
      ? loaded.gradleEnvs.filter((env) => env && typeof env.path === "string")
      : []
    settings.shortcuts =
      loaded.shortcuts && typeof loaded.shortcuts === "object"
        ? { ...loaded.shortcuts }
        : {}
  } catch {
    // Not running inside Tauri; keep defaults.
  }
}

export async function saveSettings(): Promise<void> {
  try {
    await invoke("save_settings", {
      settings: {
        closeBehavior: settings.closeBehavior,
        workspacePath: settings.workspacePath,
        themeMode: settings.themeMode,
        gradleEnvs: settings.gradleEnvs,
        shortcuts: settings.shortcuts,
      },
    })
  } catch {
    // Not running inside Tauri.
  }
}

export async function getDataDir(): Promise<string> {
  try {
    return await invoke<string>("get_data_dir")
  } catch {
    return ""
  }
}

export async function setDataDir(path: string): Promise<string> {
  return invoke<string>("set_data_dir", { path })
}
