import { generateUuid } from "./queues"
import { formatNow } from "./templates"

/** Where a config queue's sub project came from. */
export type ConfigProjectSource = "imported" | "disk"

export interface ConfigProject {
  uuid: string
  name: string
  source: ConfigProjectSource
  /** Package name when the project comes from the import area. */
  packageName?: string
  /** Project directory: the imported package folder or the picked path. */
  rootPath: string
  /** Original content location (import area folder or picked directory);
   * used by the record actions. */
  sourcePath: string
  createdAt: string
  /** Selected template name (from the templates page). */
  templateName?: string
  /** Modify-config time recorded when the template is saved. */
  configTime?: string
  /** Config start time; present once the configuration was started. */
  startedAt?: string
  /** Whether the configuration was started; started projects show in the
   * project directory. */
  started: boolean
  /** Whether the contents were copied into the config area (recorded). */
  recorded: boolean
  /** Whether the template's code folder was already copied into the
   * project directory; the next launch skips the copy step. */
  codeCopied: boolean
}

export interface ConfigQueue {
  name: string
  uuid: string
  queueType: string
  createdAt: string
  /** Sub projects embedded in their owning queue. */
  projects: ConfigProject[]
}

export interface NewConfigQueue {
  name: string
  uuid: string
  queueType: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function listConfigQueues(): Promise<ConfigQueue[]> {
  try {
    return await invoke<ConfigQueue[]>("list_config_queues")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Throws with a user-facing Chinese message on failure. */
export async function createConfigQueue(
  data: NewConfigQueue
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("create_config_queue", {
    name: data.name,
    uuid: data.uuid,
    queueType: data.queueType,
    createdAt: formatNow(),
  })
}

/** Attaches a sub project to the queue; uuid and timestamp are generated
 * locally. Only the card info is recorded — the content copy happens
 * through the explicit record actions. Throws with a user-facing Chinese
 * message on duplicates. */
export async function addConfigProject(
  queueUuid: string,
  data: Omit<
    ConfigProject,
    | "uuid"
    | "createdAt"
    | "started"
    | "startedAt"
    | "templateName"
    | "configTime"
    | "sourcePath"
    | "recorded"
    | "codeCopied"
  >
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("add_config_project", {
    queueUuid,
    project: {
      ...data,
      uuid: generateUuid(),
      createdAt: formatNow(),
      started: false,
    },
  })
}

/** Detaches a sub project from its queue; files are never touched. */
export async function removeConfigProject(
  queueUuid: string,
  projectUuid: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("remove_config_project", {
    queueUuid,
    projectUuid,
  })
}

/** Saves the selected template and the modify-config time for one sub
 * project without starting the configuration. */
export async function saveConfigTemplate(
  queueUuid: string,
  projectUuid: string,
  templateName: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("save_config_template", {
    queueUuid,
    projectUuid,
    templateName,
    configTime: formatNow(),
  })
}

/** Marks a sub project as configured and records the start time;
 * requires a previously saved template. */
export async function startConfigProject(
  queueUuid: string,
  projectUuid: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("start_config_project", {
    queueUuid,
    projectUuid,
    startedAt: formatNow(),
  })
}

/** 从模板重置代码: overwrites the project's files with the template's
 * `code` folder contents; parameter JSON untouched, no kernel runs.
 * Resolves to a user-facing Chinese summary. */
export async function resetProjectCode(projectUuid: string): Promise<string> {
  return invoke<string>("reset_project_code", { projectUuid })
}

/** Deletes queues by uuid; works for single and batch deletion. The
 * embedded project records are removed with their queue; copied folders
 * are kept. */
export async function deleteConfigQueues(uuids: string[]): Promise<void> {
  return invoke("delete_config_queues", { uuids })
}

/** Deletes sub projects by uuid across every queue; works for single and
 * batch deletion. Each project's copied folder under the config area is
 * removed too. */
export async function deleteConfigProjects(
  projectUuids: string[]
): Promise<void> {
  return invoke("delete_config_projects", { projectUuids })
}

/** Updates a disk (non-imported) project's name and package name; the
 * copied folder is renamed to match the new package name. */
export async function updateConfigProject(
  queueUuid: string,
  projectUuid: string,
  name: string,
  packageName: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("update_config_project", {
    queueUuid,
    projectUuid,
    newName: name,
    newPackageName: packageName,
  })
}

/** Clears the imported project's copied folder and copies the import
 * area's contents in again. */
export async function reloadConfigProject(
  queueUuid: string,
  projectUuid: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("reload_config_project", {
    queueUuid,
    projectUuid,
  })
}

/** Records one sub project: copies its contents into the config area
 * (imported projects from the import area, disk projects from their disk
 * directory) and marks it as recorded. */
export async function recordConfigProject(
  queueUuid: string,
  projectUuid: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("record_config_project", {
    queueUuid,
    projectUuid,
  })
}

/** Records every project of the queue; already-recorded projects are
 * refreshed. */
export async function recordAllConfigProjects(
  queueUuid: string
): Promise<ConfigQueue> {
  return invoke<ConfigQueue>("record_all_config_projects", { queueUuid })
}

/** Reads the project's parameter JSON (config/parameter/<package
 * name>.json) for in-card editing. */
export async function readProjectParameter(
  projectUuid: string
): Promise<string> {
  return invoke<string>("read_project_parameter", { projectUuid })
}

/** Re-copies the selected template's parameter JSON from the templates
 * page and returns the fresh content (overwrites the current file). */
export async function refreshProjectParameter(
  projectUuid: string
): Promise<string> {
  return invoke<string>("refresh_project_parameter", { projectUuid })
}

/** Launches a recorded project: copies the template's code folder into
 * the project's config directory (overwriting same-name files), then runs
 * the argument kernel followed by the code kernel with the package-named
 * parameter JSON. Resolves with a human-readable summary. */
export async function executeConfigProject(
  projectUuid: string
): Promise<string> {
  return invoke<string>("execute_config_project", { projectUuid })
}

/** Saves edited parameter JSON content (validated server-side); entries
 * of other write modes are kept untouched for the code kernel. */
export async function writeProjectParameter(
  projectUuid: string,
  content: string
): Promise<void> {
  return invoke("write_project_parameter", { projectUuid, content })
}
