export interface TemplateInfo {
  name: string
  templateType: string
  description: string
  createdAt: string
  updatedAt: string
  codeImported: boolean
  parameterImported: boolean
}

export interface NewTemplate {
  name: string
  templateType: string
  description: string
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

/** Local time formatted as `YYYY-MM-DD HH:mm:ss`. */
export function formatNow(): string {
  const pad = (value: number) => String(value).padStart(2, "0")
  const now = new Date()
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`
}

export async function listTemplates(): Promise<TemplateInfo[]> {
  try {
    return await invoke<TemplateInfo[]>("list_templates")
  } catch {
    // Not running inside Tauri or workspace not configured.
    return []
  }
}

/** Throws with a user-facing Chinese message on failure. */
export async function createTemplate(data: NewTemplate): Promise<TemplateInfo> {
  const now = formatNow()
  return invoke<TemplateInfo>("create_template", {
    name: data.name,
    templateType: data.templateType,
    description: data.description,
    createdAt: now,
    updatedAt: now,
  })
}

/** Updates the template identified by `originalName`. */
export async function updateTemplate(
  originalName: string,
  data: NewTemplate
): Promise<TemplateInfo> {
  return invoke<TemplateInfo>("update_template", {
    originalName,
    name: data.name,
    templateType: data.templateType,
    description: data.description,
    updatedAt: formatNow(),
  })
}

/** Deletes templates by name; works for single and batch deletion. */
export async function deleteTemplates(names: string[]): Promise<void> {
  return invoke("delete_templates", { names })
}

/** Returns the on-disk directory of a template. */
export async function getTemplateDir(name: string): Promise<string> {
  return invoke<string>("get_template_dir", { name })
}

/** Opens a path in the platform file manager. */
export async function openInExplorer(path: string): Promise<void> {
  return invoke("open_in_explorer", { path })
}

/** Copies the contents of `sourceDir` into the template's code folder. */
export async function importCodeTemplate(
  name: string,
  sourceDir: string
): Promise<void> {
  return invoke("import_code_template", { name, sourceDir })
}

/** Copies a JSON file into the template's parameter folder, renamed. */
export async function importParameterTemplate(
  name: string,
  sourceFile: string
): Promise<void> {
  return invoke("import_parameter_template", { name, sourceFile })
}

/** Reads the template's parameter JSON content for in-app editing. */
export async function readParameterJson(name: string): Promise<string> {
  return invoke<string>("read_parameter_json", { name })
}

/** Saves edited parameter JSON content (validated server-side). */
export async function writeParameterJson(
  name: string,
  content: string
): Promise<void> {
  return invoke("write_parameter_json", { name, content })
}
