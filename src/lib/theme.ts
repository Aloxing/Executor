import type { ThemeMode } from "./settings"

let mediaQuery: MediaQueryList | null = null
let mediaListener: ((event: MediaQueryListEvent) => void) | null = null

function setDarkClass(dark: boolean) {
  document.documentElement.classList.toggle("dark", dark)
}

function detachSystemListener() {
  if (mediaQuery && mediaListener) {
    mediaQuery.removeEventListener("change", mediaListener)
  }
  mediaQuery = null
  mediaListener = null
}

/**
 * Applies the given theme mode to the document. "system" follows the OS
 * preference live; explicit modes also persist to localStorage so the
 * inline script in index.html can apply them before first paint.
 */
export function applyThemeMode(mode: ThemeMode) {
  detachSystemListener()
  if (mode === "system") {
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)")
    mediaListener = (event) => setDarkClass(event.matches)
    mediaQuery.addEventListener("change", mediaListener)
    setDarkClass(mediaQuery.matches)
    try {
      // Let the pre-paint script fall back to the OS preference.
      localStorage.removeItem("theme")
    } catch {
      // Storage unavailable; theme still applies.
    }
  } else {
    setDarkClass(mode === "dark")
    try {
      localStorage.setItem("theme", mode)
    } catch {
      // Storage unavailable (e.g. privacy mode); theme still applies.
    }
  }
}
