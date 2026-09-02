import { generateUuid } from "./queues"
import { formatNow } from "./templates"

/** Where a build project came from. */
export type BuildProjectSource = "config" | "disk"

export interface BuildProject {
  uuid: string
  name: string
  source: BuildProjectSource
  /** Package name of a config-area project; absent for disk projects. */
  packageName?: string
  /** Project directory the build commands run in. */
  rootPath: string
  createdAt: string
}

export interface BuildQueue {
  name: string
  uuid: string
  queueType: string
  createdAt: string
  projects: BuildProject[]
}

export interface NewBuildQueue {
  name: string
  uuid: string
  queueType: string
}

/** A configured project of the config area picked for building. */
export interface ConfiguredPick {
  name: string
  packageName: string
  rootPath: string
}

/** Gradle task presets offered by the context menus; every build runs
 * `<gradle env> wrapper` first and then `gradlew <args>` on success. */
export const buildCommands: { label: string; args: string[] }[] = [
  { label: "assembleDebug", args: ["assembleDebug"] },
  { label: "assembleRelease", args: ["assembleRelease"] },
  { label: "clean assembleRelease", args: ["clean", "assembleRelease"] },
]

/** One streamed log chunk of the `build-log` event; `line` may contain
 * several `\n`-joined lines (the backend coalesces output). */
export interface BuildLogEvent {
  projectUuid: string
  /** `status` (flow markers), `stdout` (command output) or `done`. */
  kind: "status" | "stdout" | "done"
  line: string
  success?: boolean
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function listBuildQueues(): Promise<BuildQueue[]> {
  try {
    return await invoke<BuildQueue[]>("list_build_queues")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Throws with a user-facing Chinese message on failure. */
export async function createBuildQueue(
  data: NewBuildQueue
): Promise<BuildQueue> {
  return invoke<BuildQueue>("create_build_queue", {
    name: data.name,
    uuid: data.uuid,
    queueType: data.queueType,
    createdAt: formatNow(),
  })
}

/** Attaches a project record (address only, nothing is copied); uuid and
 * timestamp are generated locally. */
export async function addBuildProject(
  queueUuid: string,
  data: Omit<BuildProject, "uuid" | "createdAt">
): Promise<BuildQueue> {
  return invoke<BuildQueue>("add_build_project", {
    queueUuid,
    project: {
      ...data,
      uuid: generateUuid(),
      createdAt: formatNow(),
    },
  })
}

/** Detaches a project record from its queue; files are never touched. */
export async function removeBuildProject(
  queueUuid: string,
  projectUuid: string
): Promise<BuildQueue> {
  return invoke<BuildQueue>("remove_build_project", {
    queueUuid,
    projectUuid,
  })
}

/** Runs the build flow of one project; output streams through the
 * `build-log` event (multi-line chunks) while the promise stays pending
 * until the end. */
export async function runProjectBuild(
  projectUuid: string,
  gradleEnvPath: string,
  taskArgs: string[]
): Promise<void> {
  return invoke("run_project_build", {
    projectUuid,
    gradleEnvPath,
    taskArgs,
  })
}

/** Stops a running build by killing its whole process tree. */
export async function stopProjectBuild(projectUuid: string): Promise<void> {
  return invoke("stop_project_build", { projectUuid })
}

/** Subscribes to the streamed build logs; resolves to the unlisten fn. */
export async function listenBuildLog(
  handler: (event: BuildLogEvent) => void
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event")
  return listen<BuildLogEvent>("build-log", (event) => handler(event.payload))
}
