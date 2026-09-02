/** Records page: every add/delete/modify operation of the import,
 * config, build and output areas is logged by the backend as one card.
 * Records persist in `<workspace>/records/records.json` (newest first,
 * capped by the backend). Deleting records never touches real files. */

/** Page tag of a record. */
export type RecordPage = "import" | "config" | "build" | "output"
/** Operation tag of a record. */
export type RecordAction = "add" | "delete" | "modify"

export interface OpRecord {
  uuid: string
  page: string
  action: string
  title: string
  detail: string
  /** Sub records: the affected entries (project names, files…). */
  items: string[]
  createdAt: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

export async function listRecords(): Promise<OpRecord[]> {
  try {
    return await invoke<OpRecord[]>("list_records")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Deletes whole record cards (single and batch share this command). */
export async function removeRecords(uuids: string[]): Promise<void> {
  return invoke("remove_records", { uuids })
}

/** Deletes one sub record by index; the card disappears with its last
 * sub record. Resolves to the updated list. */
export async function removeRecordItem(
  uuid: string,
  index: number
): Promise<OpRecord[]> {
  return invoke<OpRecord[]>("remove_record_item", { uuid, index })
}
