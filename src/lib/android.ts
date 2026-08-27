import { formatNow } from "@/lib/templates"

/** Import status of an Android project; persisted in android.json. */
export type ImportStatus = "pending" | "importing" | "imported"

export interface AndroidProject {
  appName: string
  packageName: string
  rootPath: string
  createdAt: string
  updatedAt: string
  importStatus: ImportStatus
  queueUuid: string
  /** Imported location (`<workspace>/import/package/<package name>`),
   * computed by the backend on read; absent when not imported yet. */
  location?: string
}

export interface NewAndroidProject {
  appName: string
  packageName: string
  rootPath: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function listAndroidProjects(): Promise<AndroidProject[]> {
  try {
    return await invoke<AndroidProject[]>("list_android_projects")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Records a new Android project under `queueUuid` (no file copy yet). */
export async function addAndroidProject(
  queueUuid: string,
  data: NewAndroidProject
): Promise<AndroidProject> {
  const now = formatNow()
  return invoke<AndroidProject>("add_android_project", {
    queueUuid,
    appName: data.appName,
    packageName: data.packageName,
    rootPath: data.rootPath,
    createdAt: now,
    updatedAt: now,
  })
}

/** Updates a project located by its original package name; the package
 * name itself may change too (the folder is renamed accordingly). A
 * changed root path is re-copied into the package folder immediately. */
export async function updateAndroidProject(
  originalPackageName: string,
  data: NewAndroidProject
): Promise<AndroidProject> {
  return invoke<AndroidProject>("update_android_project", {
    packageName: originalPackageName,
    newPackageName: data.packageName,
    appName: data.appName,
    rootPath: data.rootPath,
    updatedAt: formatNow(),
  })
}

/** Returns the imported project's folder for locating it in Explorer. */
export async function getAndroidProjectDir(
  packageName: string
): Promise<string> {
  return invoke<string>("get_android_project_dir", { packageName })
}

/** Clears the imported contents and re-imports from the download path. */
export async function reloadAndroidProject(
  packageName: string
): Promise<AndroidProject> {
  return invoke<AndroidProject>("reload_android_project", { packageName })
}

/** Deletes the record and the on-disk package folder. */
export async function deleteAndroidProject(packageName: string): Promise<void> {
  return invoke("delete_android_project", { packageName })
}

/** Deletes projects by package name; works for batch deletion. */
export async function deleteAndroidProjects(
  packageNames: string[]
): Promise<void> {
  return invoke("delete_android_projects", { packageNames })
}

/** Detaches the project from its queue; record and folder are kept. */
export async function detachAndroidProject(packageName: string): Promise<void> {
  return invoke("detach_android_project", { packageName })
}

/** Copies every project of the queue into its package folder and marks
 * them as imported. */
export async function importAndroidProjects(
  queueUuid: string
): Promise<AndroidProject[]> {
  return invoke<AndroidProject[]>("import_android_projects", { queueUuid })
}
