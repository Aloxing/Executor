import { formatNow } from "@/lib/templates"

export interface ImportQueue {
  name: string
  uuid: string
  queueType: string
  createdAt: string
  /** Package names of the attached Android projects. */
  packages: string[]
}

export interface NewImportQueue {
  name: string
  uuid: string
  queueType: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

/** Generates a v4 UUID, with a Math.random fallback for contexts where
 * `crypto.randomUUID` is unavailable. */
export function generateUuid(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID()
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const random = (Math.random() * 16) | 0
    const value = c === "x" ? random : (random & 0x3) | 0x8
    return value.toString(16)
  })
}

export async function listImportQueues(): Promise<ImportQueue[]> {
  try {
    return await invoke<ImportQueue[]>("list_import_queues")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Throws with a user-facing Chinese message on failure. */
export async function createImportQueue(
  data: NewImportQueue
): Promise<ImportQueue> {
  return invoke<ImportQueue>("create_import_queue", {
    name: data.name,
    uuid: data.uuid,
    queueType: data.queueType,
    createdAt: formatNow(),
  })
}

/** Deletes queues by uuid; works for single and batch deletion. Attached
 * Android projects are kept (detached) in the import directory. */
export async function deleteQueues(uuids: string[]): Promise<void> {
  return invoke("delete_queues", { uuids })
}
