/** Cross-page navigation for the automation pipeline: any view can ask
 * App.vue to switch the active page (e.g. after forwarding a queue). */

export type ViewKey =
  | "import"
  | "config"
  | "build"
  | "output"
  | "records"
  | "templates"

export function navigateTo(view: ViewKey) {
  window.dispatchEvent(new CustomEvent("app-navigate", { detail: view }))
}
