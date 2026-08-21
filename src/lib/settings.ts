import { reactive } from "vue"

export type CloseBehavior = "ask" | "tray" | "exit"
export type ThemeMode = "dark" | "light" | "system"

export interface AppSettings {
  closeBehavior: CloseBehavior
  workspacePath: string
  themeMode: ThemeMode
}

const defaults: AppSettings = {
  closeBehavior: "ask",
  workspacePath: "",
  themeMode: "system",
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
