<script setup lang="ts">
import { Moon, Sun } from "lucide-vue-next"
import { onMounted, onUnmounted, ref } from "vue"
import appIcon from "@/assets/app-icon.png"
import CloseDialog, { type CloseAction } from "./CloseDialog.vue"
import { saveSettings, settings } from "@/lib/settings"
import { applyThemeMode } from "@/lib/theme"

type WindowApi = Awaited<
  ReturnType<typeof import("@tauri-apps/api/window")["getCurrentWindow"]>
>

let appWindow: WindowApi | null = null
let unlistenResized: (() => void) | undefined
let disposed = false
const isMaximized = ref(false)
const showCloseDialog = ref(false)

async function ensureWindow() {
  if (!appWindow) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window")
    appWindow = getCurrentWindow()
  }
  return appWindow
}

async function syncMaximized() {
  try {
    const win = await ensureWindow()
    isMaximized.value = await win.isMaximized()
  } catch {
    isMaximized.value = false
  }
}

async function minimize() {
  try {
    const win = await ensureWindow()
    await win.minimize()
  } catch {
    // Not running inside Tauri.
  }
}

async function toggleMaximize() {
  try {
    const win = await ensureWindow()
    await win.toggleMaximize()
    await syncMaximized()
  } catch {
    // Not running inside Tauri.
  }
}

async function runCloseAction(action: CloseAction) {
  try {
    const { invoke } = await import("@tauri-apps/api/core")
    if (action === "tray") {
      // Hide the window and create the tray icon on demand; the tray icon
      // does not exist while the window is visible.
      await invoke("hide_to_tray")
    } else {
      await invoke("exit_app")
    }
  } catch {
    // Not running inside Tauri.
  }
}

async function requestClose() {
  // The persisted close behavior decides whether we ask first.
  if (settings.closeBehavior === "tray" || settings.closeBehavior === "exit") {
    await runCloseAction(settings.closeBehavior)
    return
  }
  showCloseDialog.value = true
}

async function handleCloseChoice(action: CloseAction, remember: boolean) {
  showCloseDialog.value = false
  if (remember) {
    settings.closeBehavior = action
    await saveSettings()
  }
  await runCloseAction(action)
}

onMounted(async () => {
  await syncMaximized()
  try {
    const win = await ensureWindow()
    const unlisten = await win.onResized(() => syncMaximized())
    if (disposed) {
      // Component was unmounted before the listener resolved; clean it up
      // immediately to avoid leaking the event listener.
      unlisten()
    } else {
      unlistenResized = unlisten
    }
  } catch {
    // Not running inside Tauri.
  }
})

onUnmounted(() => {
  disposed = true
  unlistenResized?.()
})

// The initial class is applied by the inline script in index.html before
// the app renders, so reading it here reflects the effective theme.
const isDark = ref(document.documentElement.classList.contains("dark"))

function toggleTheme() {
  isDark.value = !isDark.value
  // The title bar toggle is an explicit choice that overrides "system".
  settings.themeMode = isDark.value ? "dark" : "light"
  applyThemeMode(settings.themeMode)
  saveSettings()
}
</script>

<template>
  <div
    data-tauri-drag-region="deep"
    class="relative flex h-10 shrink-0 select-none items-center bg-transparent"
  >
    <div
      class="flex min-w-0 flex-1 items-center justify-between gap-2 pl-3 pr-2"
    >
      <div class="flex min-w-0 items-center gap-2">
        <img :src="appIcon" class="size-4" alt="" />
        <slot />
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <button
          type="button"
          class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          :aria-label="isDark ? '切换到亮色模式' : '切换到暗色模式'"
          :title="isDark ? '切换到亮色模式' : '切换到暗色模式'"
          @click="toggleTheme"
        >
          <Sun
            v-if="isDark"
            key="sun"
            class="animate-in fade-in zoom-in-75 size-3.5 duration-200"
          />
          <Moon
            v-else
            key="moon"
            class="animate-in fade-in zoom-in-75 size-3.5 duration-200"
          />
        </button>
        <span aria-hidden="true" class="bg-border mx-0.5 h-3.5 w-px shrink-0" />
        <button
          type="button"
          class="hover:bg-accent hover:text-accent-foreground inline-flex h-7 w-8 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
          aria-label="最小化"
          @click="minimize"
        >
          <span aria-hidden="true" class="win-caption-glyph">&#xE921;</span>
        </button>
        <button
          type="button"
          class="hover:bg-accent hover:text-accent-foreground inline-flex h-7 w-8 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
          :aria-label="isMaximized ? '还原' : '最大化'"
          @click="toggleMaximize"
        >
          <span
            v-if="!isMaximized"
            aria-hidden="true"
            class="win-caption-glyph"
          >
            &#xE922;
          </span>
          <span v-else aria-hidden="true" class="win-caption-glyph">
            &#xE923;
          </span>
        </button>
        <button
          type="button"
          class="hover:bg-destructive/10 hover:text-destructive inline-flex h-7 w-8 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
          aria-label="关闭"
          @click="requestClose"
        >
          <span aria-hidden="true" class="win-caption-glyph">&#xE8BB;</span>
        </button>
      </div>
    </div>
    <CloseDialog
      v-if="showCloseDialog"
      @select="handleCloseChoice"
      @cancel="showCloseDialog = false"
    />
  </div>
</template>
