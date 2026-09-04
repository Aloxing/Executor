/**
 * Scene code bridge: the two-way mapping between a `write_mode == "code"`
 * entry's `scenes` JSON and the equivalent Java methods.
 *
 * Generation mirrors the Rust code kernel (`core/android/code/mod.rs`:
 * `generate_code` / `generate_method`) statement for statement, so the
 * preview shows exactly what the kernel will inject. Parsing is the
 * inverse: it rebuilds `scenes` from method source, and reports every
 * line it cannot map back so the caller can refuse a lossy write.
 */

/** One body statement: `direct` calls the callback, `ruled` wraps it in a
 * rule template call (`Class.method(ruleArgs, this::callback)`). */
export interface SceneStatement {
  type: string
  call?: { callback?: string; args?: unknown[] }
  rule?: { template?: string; args?: unknown[] }
}

export interface SceneParam {
  type: string
  name: string
}

export interface SceneDef {
  returnType?: string
  params?: SceneParam[]
  body?: SceneStatement[]
  return?: string
}

export type Scenes = Record<string, SceneDef>

/** Parse outcome: the rebuilt scenes plus every line that could not be
 * mapped back (empty means the text is fully representable in JSON). */
export interface SceneParseResult {
  scenes: Scenes
  /** `logTag` read from the `Log.i(...)` lines; empty when absent. */
  logTag: string
  errors: string[]
}

// ----------------------------------------------------------------------
// Argument tokens (shared with the 实参 inputs of the parameter card)
// ----------------------------------------------------------------------

/** Comma splitter that keeps quoted segments intact, so `"a,b"` stays one
 * argument instead of breaking on the inner comma. */
function splitArgTokens(text: string): string[] {
  const tokens: string[] = []
  let current = ""
  let quote = ""
  for (const ch of text) {
    if (quote) {
      current += ch
      if (ch === quote) quote = ""
      continue
    }
    if (ch === '"' || ch === "'") {
      quote = ch
      current += ch
      continue
    }
    if (ch === ",") {
      tokens.push(current)
      current = ""
      continue
    }
    current += ch
  }
  tokens.push(current)
  return tokens
}

/** Renders one argument the way the kernel's `format_args_list` does:
 * strings are quoted, `{"var": name}` renders as a bare variable
 * reference, everything else uses its plain text form. */
export function renderArgToken(arg: unknown): string {
  if (arg !== null && typeof arg === "object" && "var" in arg) {
    return String((arg as { var: unknown }).var ?? "")
  }
  if (typeof arg === "string") return `"${arg}"`
  return String(arg)
}

export function argsText(args: unknown): string {
  return Array.isArray(args) ? args.map(renderArgToken).join(",") : ""
}

/** Argument list as the kernel writes it into generated code: `", "`
 * separated (`format_args_list`), unlike the compact `argsText` of the
 * 实参 inputs. Both forms parse back identically. */
export function argsCode(args: unknown): string {
  return Array.isArray(args) ? args.map(renderArgToken).join(", ") : ""
}

/** Inverse of `argsText`: quoted tokens are strings, pure numbers stay
 * numbers, bare identifiers become variables (`{"var": name}`). */
export function parseArgs(text: string): unknown[] {
  if (!text) return []
  const tokens: unknown[] = []
  for (const raw of splitArgTokens(text)) {
    const trimmed = raw.trim()
    // Empty tokens mean the argument was deleted; drop them so the input
    // never bounces back to a re-rendered "" (type two quotes for a real
    // empty-string argument).
    if (!trimmed) continue
    // Quoted tokens are strings.
    if (
      trimmed.length >= 2 &&
      ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'")))
    ) {
      tokens.push(trimmed.slice(1, -1))
      continue
    }
    // Pure numbers render without quotes.
    if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
      tokens.push(Number(trimmed))
      continue
    }
    // Bare identifiers are variables.
    tokens.push({ var: trimmed })
  }
  return tokens
}

// ----------------------------------------------------------------------
// Generation (JSON -> Java), mirroring the Rust kernel
// ----------------------------------------------------------------------

function str(value: unknown): string {
  return typeof value === "string" ? value.trim() : ""
}

/** The rule template's call target, with the kernel's defaults: class
 * `advertiseComplianceJob` and the template name as the method. */
export function resolveRule(
  ruleTemplates: Record<string, any>,
  template: string
): { objClass: string; method: string } {
  const def = ruleTemplates?.[template]
  const objClass = str(def?.class) || "advertiseComplianceJob"
  const method = str(def?.method) || template
  return { objClass, method }
}

/** Effective log tag: the entry's `logTag`, or the kernel's `TAG`. */
export function effectiveLogTag(entry: Record<string, any> | undefined): string {
  return str(entry?.logTag) || "TAG"
}

/**
 * Lists the statements the kernel skips (a `type` other than direct/ruled).
 * They have no Java form, so a code-mode edit of their function cannot
 * preserve them — the caller warns before the user loses anything.
 */
export function unsupportedStatements(
  entry: Record<string, any> | undefined
): string[] {
  const scenes: Scenes =
    entry?.scenes && typeof entry.scenes === "object" && !Array.isArray(entry.scenes)
      ? entry.scenes
      : {}
  const found: string[] = []
  for (const name of Object.keys(scenes)) {
    const body = Array.isArray(scenes[name]?.body) ? scenes[name].body! : []
    body.forEach((stmt, index) => {
      const type = str(stmt?.type) || "direct"
      if (type !== "direct" && type !== "ruled") {
        found.push(`函数「${name}」第 ${index + 1} 条语句（type=${type}）`)
      }
    })
  }
  return found
}

function renderParams(scene: SceneDef | undefined): string {
  const params = Array.isArray(scene?.params) ? scene!.params! : []
  return params
    .map((param) => ({ type: str(param?.type), name: str(param?.name) }))
    .filter((param) => param.type && param.name)
    .map((param) => `${param.type} ${param.name}`)
    .join(", ")
}

function renderStatement(
  stmt: SceneStatement,
  ruleTemplates: Record<string, any>
): string | null {
  const type = str(stmt?.type) || "direct"
  const callback = str(stmt?.call?.callback)
  const callArgs = argsCode(stmt?.call?.args)
  if (type === "direct") {
    return `${callback}(${callArgs});`
  }
  if (type === "ruled") {
    const template = str(stmt?.rule?.template)
    const { objClass, method } = resolveRule(ruleTemplates, template)
    const ruleArgs = argsCode(stmt?.rule?.args)
    const cbRef = `this::${callback}`
    const allArgs = ruleArgs ? `${ruleArgs}, ${cbRef}` : cbRef
    return `${objClass}.${method}(${allArgs});`
  }
  // Unknown statement types are skipped by the kernel as well.
  return null
}

/** One scene as Java source — byte-identical to the kernel's output. */
export function generateMethodCode(
  sceneName: string,
  scene: SceneDef | undefined,
  ruleTemplates: Record<string, any>,
  logTag: string
): string {
  const returnType = str(scene?.returnType) || "void"
  const lines = [
    `    public ${returnType} ${sceneName}(${renderParams(scene)}) {`,
    `        Log.i(${logTag}, "${sceneName}: ");`,
  ]
  for (const stmt of Array.isArray(scene?.body) ? scene!.body! : []) {
    const rendered = renderStatement(stmt, ruleTemplates)
    if (rendered) lines.push(`        ${rendered}`)
  }
  const returnExpr = str(scene?.return)
  if (returnExpr) lines.push(`        return ${returnExpr};`)
  lines.push("    }")
  return lines.join("\n")
}

/** Every scene of one code entry, joined like the kernel does. */
export function generateScenesCode(entry: Record<string, any> | undefined): string {
  const scenes: Scenes = entry?.scenes && typeof entry.scenes === "object" ? entry.scenes : {}
  const ruleTemplates =
    entry?.ruleTemplates && typeof entry.ruleTemplates === "object"
      ? entry.ruleTemplates
      : {}
  const logTag = effectiveLogTag(entry)
  return Object.keys(scenes)
    .map((name) => generateMethodCode(name, scenes[name], ruleTemplates, logTag))
    .join("\n\n")
}

// ----------------------------------------------------------------------
// Parsing (Java -> JSON)
// ----------------------------------------------------------------------

const METHOD_START = /^\s*public\s+(.*?)\s+(\w+)\s*\(([^()]*)\)\s*\{\s*$/
const LOG_LINE = /^Log\.i\(\s*([^,]+?)\s*,/
const RETURN_LINE = /^return\s+(.+?)\s*;$/
const RULED_LINE = /^([\w$.]+)\.(\w+)\s*\((.*)\)\s*;$/
const DIRECT_LINE = /^(\w+)\s*\((.*)\)\s*;$/

/** Net brace depth of one line, ignoring braces inside string literals. */
function braceDelta(line: string): number {
  let delta = 0
  let quote = ""
  for (const ch of line) {
    if (quote) {
      if (ch === quote) quote = ""
      continue
    }
    if (ch === '"' || ch === "'") {
      quote = ch
      continue
    }
    if (ch === "{") delta += 1
    else if (ch === "}") delta -= 1
  }
  return delta
}

/** Splits a formal parameter list on top-level commas (generics such as
 * `Map<String, Object> map` stay one parameter). */
function splitParams(text: string): string[] {
  const parts: string[] = []
  let current = ""
  let depth = 0
  for (const ch of text) {
    if (ch === "<" || ch === "(" || ch === "[") depth += 1
    else if (ch === ">" || ch === ")" || ch === "]") depth -= 1
    if (ch === "," && depth === 0) {
      parts.push(current)
      current = ""
      continue
    }
    current += ch
  }
  parts.push(current)
  return parts
}

function parseParams(text: string): SceneParam[] {
  const params: SceneParam[] = []
  for (const raw of splitParams(text)) {
    const trimmed = raw.trim()
    if (!trimmed) continue
    const split = trimmed.split(/\s+/)
    const name = split.pop() ?? ""
    const type = split.join(" ")
    if (name && type) params.push({ type, name })
  }
  return params
}

/** Finds the rule template whose call target matches `Class.method`. */
function findTemplate(
  ruleTemplates: Record<string, any>,
  objClass: string,
  method: string
): string {
  for (const name of Object.keys(ruleTemplates ?? {})) {
    const resolved = resolveRule(ruleTemplates, name)
    if (resolved.objClass === objClass && resolved.method === method) return name
  }
  return ""
}

/** Parses method source back into scenes. Every line that cannot be
 * represented in the schema lands in `errors` (with its 1-based line
 * number) so the caller never writes a lossy result. */
export function parseScenesCode(
  text: string,
  ruleTemplates: Record<string, any>
): SceneParseResult {
  const scenes: Scenes = {}
  const errors: string[] = []
  let logTag = ""
  const lines = text.split("\n")

  let index = 0
  while (index < lines.length) {
    const line = lines[index]
    // Blank lines and the blank line between methods carry no meaning.
    if (!line.trim()) {
      index += 1
      continue
    }
    const signature = METHOD_START.exec(line)
    if (!signature) {
      // Anything outside a method body cannot be represented in scenes.
      if (line.trim() !== "}") {
        errors.push(`第 ${index + 1} 行不是可识别的函数声明：${line.trim()}`)
      }
      index += 1
      continue
    }

    const returnType = signature[1].trim()
    const sceneName = signature[2]
    const params = parseParams(signature[3])
    if (scenes[sceneName]) {
      errors.push(`第 ${index + 1} 行函数「${sceneName}」重名，请改用唯一名称`)
    }

    // Collect the body up to the matching closing brace (depth counting,
    // the same approach the kernel uses to drop same-named methods).
    const body: SceneStatement[] = []
    let returnExpr = ""
    let depth = braceDelta(line)
    let closed = depth === 0
    let cursor = index + 1
    for (; cursor < lines.length; cursor++) {
      const raw = lines[cursor]
      const trimmed = raw.trim()
      depth += braceDelta(raw)
      if (depth <= 0) {
        closed = true
        break
      }
      if (!trimmed) continue
      const logMatch = LOG_LINE.exec(trimmed)
      if (logMatch) {
        // The kernel writes this line itself; only the tag is of interest.
        if (!logTag) logTag = logMatch[1].trim()
        continue
      }
      const returnMatch = RETURN_LINE.exec(trimmed)
      if (returnMatch) {
        returnExpr = returnMatch[1].trim()
        continue
      }
      const statement = parseStatement(trimmed, ruleTemplates)
      if (statement.error) {
        errors.push(`第 ${cursor + 1} 行${statement.error}`)
        continue
      }
      body.push(statement.statement!)
    }
    if (!closed) {
      errors.push(`函数「${sceneName}」缺少结束的 }`)
    }

    const scene: SceneDef = { returnType, body }
    if (params.length) scene.params = params
    if (returnExpr) scene.return = returnExpr
    scenes[sceneName] = scene
    index = cursor + 1
  }

  return { scenes, logTag, errors }
}

/** One body line as a statement; `error` explains anything unsupported. */
function parseStatement(
  trimmed: string,
  ruleTemplates: Record<string, any>
): { statement?: SceneStatement; error?: string } {
  const ruled = RULED_LINE.exec(trimmed)
  if (ruled) {
    const [, objClass, method, argsTextRaw] = ruled
    const marker = argsTextRaw.lastIndexOf("this::")
    if (marker < 0) {
      return {
        error: `无法解析（规则调用需以 this::回调 结尾）：${trimmed}`,
      }
    }
    const callback = argsTextRaw.slice(marker + "this::".length).trim()
    if (!/^\w+$/.test(callback)) {
      return { error: `无法解析的回调名称：${trimmed}` }
    }
    const ruleArgsRaw = argsTextRaw.slice(0, marker).trim().replace(/,$/, "").trim()
    const template = findTemplate(ruleTemplates, objClass, method)
    if (!template) {
      return {
        error: `找不到与 ${objClass}.${method} 对应的规则模板：${trimmed}`,
      }
    }
    return {
      statement: {
        type: "ruled",
        rule: { template, args: parseArgs(ruleArgsRaw) },
        call: { callback, args: [] },
      },
    }
  }

  const direct = DIRECT_LINE.exec(trimmed)
  if (direct) {
    return {
      statement: {
        type: "direct",
        call: { callback: direct[1], args: parseArgs(direct[2]) },
      },
    }
  }

  return { error: `无法解析为 direct / ruled 语句：${trimmed}` }
}

// ----------------------------------------------------------------------
// Applying a parse result without churning untouched scenes
// ----------------------------------------------------------------------

/**
 * Merges parsed scenes over the existing ones. A scene whose generated
 * code is unchanged keeps its original object (so untouched entries are
 * never rewritten — no added `returnType`, no reordered keys), while a
 * changed scene is rebuilt from the parse, keeping unknown extra fields.
 */
export function applyScenes(
  existing: Scenes,
  parsed: SceneParseResult,
  ruleTemplates: Record<string, any>,
  logTag: string
): Scenes {
  const next: Scenes = {}
  for (const name of Object.keys(parsed.scenes)) {
    const fresh = parsed.scenes[name]
    const old = existing?.[name]
    if (old && typeof old === "object") {
      const sameCode =
        generateMethodCode(name, old, ruleTemplates, logTag) ===
        generateMethodCode(name, fresh, ruleTemplates, logTag)
      if (sameCode) {
        next[name] = old
        continue
      }
      // Keep unknown extra fields of the edited scene.
      next[name] = { ...(old as Record<string, unknown>), ...fresh } as SceneDef
      const merged = next[name] as Record<string, unknown>
      // Fields the parse dropped must not survive from the old scene.
      if (!fresh.params) delete merged.params
      if (!fresh.return) delete merged.return
      // An omitted return type stays omitted when it is the default.
      if (fresh.returnType === "void" && !("returnType" in (old as object))) {
        delete merged.returnType
      }
      continue
    }
    const scene = { ...fresh } as Record<string, unknown>
    // New functions only carry a return type when it is not the default.
    if (scene.returnType === "void") delete scene.returnType
    next[name] = scene as SceneDef
  }
  return next
}
