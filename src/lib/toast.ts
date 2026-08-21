import { reactive } from "vue"

export type ToastType = "error" | "success" | "info"

export interface Toast {
  id: number
  type: ToastType
  message: string
}

/** At most this many toasts are visible so they never fill the screen. */
const MAX_TOASTS = 5
const DURATION = 4000

const state = reactive<{ toasts: Toast[] }>({ toasts: [] })

let nextId = 1

export const toasts = state.toasts

/** Shows a toast in the bottom-right corner; auto-dismisses after 4s. */
export function showToast(message: string, type: ToastType = "error") {
  const id = nextId++
  state.toasts.push({ id, type, message })
  // Drop the oldest ones beyond the cap instead of overflowing the screen.
  if (state.toasts.length > MAX_TOASTS) {
    state.toasts.splice(0, state.toasts.length - MAX_TOASTS)
  }
  setTimeout(() => dismissToast(id), DURATION)
}

export function dismissToast(id: number) {
  const index = state.toasts.findIndex((t) => t.id === id)
  if (index >= 0) state.toasts.splice(index, 1)
}
