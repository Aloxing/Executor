/** Output area: artifact cards recorded automatically on every
 * successful build. The artifact files stay where the build produced
 * them; deletes are real file deletions, copies go to a user-picked
 * destination. Records persist in `<workspace>/output/outputs.json`. */

export interface OutputFile {
  name: string
  path: string
}

export interface OutputRecord {
  uuid: string
  projectName: string
  /** Package name when known (config-area projects). */
  packageName?: string | null
  /** Template name shown as the card tag; absent for direct disk builds. */
  templateName?: string | null
  /** Where the project info came from: `config` or `build`. */
  infoSource: string
  /** Build type; only Android for now. */
  buildType: string
  /** Project directory the build ran in. */
  rootPath: string
  files: OutputFile[]
  /** When the artifacts were recorded (build success time). */
  createdAt: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function listOutputs(): Promise<OutputRecord[]> {
  try {
    return await invoke<OutputRecord[]>("list_outputs")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Really deletes the artifact files of the given cards (single and
 * batch deletion share this command). */
export async function removeOutputs(uuids: string[]): Promise<void> {
  return invoke("remove_outputs", { uuids })
}

/** Really deletes one artifact file; the card disappears with its last
 * file. Resolves to the updated list. */
export async function removeOutputFile(
  uuid: string,
  filePath: string
): Promise<OutputRecord[]> {
  return invoke<OutputRecord[]>("remove_output_file", { uuid, filePath })
}

/** Copies one artifact file to `dest` (picked through a save dialog). */
export async function copyOutputFile(src: string, dest: string): Promise<void> {
  return invoke("copy_output_file", { src, dest })
}
