import { formatNow } from "./templates"

export interface WorkspaceArea {
  name: string
  label: string
  folder: string
  createdAt: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

/**
 * Creates the page area folders (import/config/build/output/records/
 * templates) in the workspace and persists the structure to workspace.json.
 * Throws with a user-facing Chinese message on failure.
 */
export async function ensureWorkspaceAreas(): Promise<WorkspaceArea[]> {
  return invoke<WorkspaceArea[]>("ensure_workspace_areas", {
    now: formatNow(),
  })
}
