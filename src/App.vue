<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue"
import AppTitleBar from "./components/AppTitleBar.vue"
import BuildView from "./components/build/BuildView.vue"
import ConfigView from "./components/config/ConfigView.vue"
import FloatingNavBar, { type NavKey } from "./components/FloatingNavBar.vue"
import ImportView from "./components/import/ImportView.vue"
import OutputView from "./components/output/OutputView.vue"
import RecordsView from "./components/records/RecordsView.vue"
import SettingsModal from "./components/SettingsModal.vue"
import TemplatesView from "./components/templates/TemplatesView.vue"
import ToastContainer from "./components/ToastContainer.vue"
import { loadSettings, settings } from "./lib/settings"
import { installShortcutDispatcher, registerShortcut } from "./lib/shortcuts"
import { applyThemeMode } from "./lib/theme"
import { ensureWorkspaceAreas } from "./lib/workspace"

const activeView = ref<NavKey>("import")
const showSettings = ref(false)

const viewLabels: Record<NavKey, string> = {
  import: "导入区",
  config: "配置区",
  build: "构建区",
  output: "产出区",
  records: "记录",
  templates: "模板",
}
const activeViewLabel = computed(() => viewLabels[activeView.value])

// App-level shortcut handlers: the single global dispatcher plus the
// navigation and open-settings actions. Page-level actions (save, close,
// search, create) are registered by the views and overlays themselves.
let disposeDispatcher: (() => void) | undefined
const disposeRegistrations: (() => void)[] = []

onMounted(() => {
  disposeDispatcher = installShortcutDispatcher()
  disposeRegistrations.push(
    registerShortcut("settings", () => {
      showSettings.value = true
    }),
    registerShortcut("nav.import", () => {
      activeView.value = "import"
    }),
    registerShortcut("nav.config", () => {
      activeView.value = "config"
    }),
    registerShortcut("nav.build", () => {
      activeView.value = "build"
    }),
    registerShortcut("nav.output", () => {
      activeView.value = "output"
    }),
    registerShortcut("nav.records", () => {
      activeView.value = "records"
    }),
    registerShortcut("nav.templates", () => {
      activeView.value = "templates"
    })
  )
})

onUnmounted(() => {
  disposeDispatcher?.()
  for (const off of disposeRegistrations) off()
})

// Implemented views are kept alive across switches so page state
// (filters, inputs, expanded cards…) survives navigation.
const viewComponent = computed(() => {
  switch (activeView.value) {
    case "templates":
      return TemplatesView
    case "import":
      return ImportView
    case "config":
      return ConfigView
    case "build":
      return BuildView
    case "output":
      return OutputView
    case "records":
      return RecordsView
    default:
      return null
  }
})

onMounted(async () => {
  await nextTick()
  // Load persisted settings (close behavior etc.) before the window shows.
  await loadSettings()
  applyThemeMode(settings.themeMode)
  // Repair/create the page area folders of an existing workspace.
  if (settings.workspacePath) {
    ensureWorkspaceAreas().catch(() => {})
  }
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window")
    const win = getCurrentWindow()
    await win.show()
    await win.setFocus()
    // Tell the Rust side that the window was shown normally, so the 1.5s
    // fallback thread will not override the user's later close-to-tray.
    const { invoke } = await import("@tauri-apps/api/core")
    await invoke("mark_window_handled")
    // The window is visible, so the on-demand tray icon (if any) should go.
    await invoke("restore_from_tray")
  } catch (error) {
    console.warn("show window failed", error)
  }
})
</script>

<template>
  <div
    class="bg-card text-card-foreground flex h-screen w-screen flex-col overflow-hidden rounded-[7px] border border-border shadow-sm"
  >
    <AppTitleBar>
      <FloatingNavBar
        @change="activeView = $event"
        @open-settings="showSettings = true"
      />
    </AppTitleBar>
    <main class="min-h-0 flex-1 overflow-auto p-[17px]">
      <KeepAlive>
        <component
          :is="viewComponent"
          v-if="viewComponent"
          :key="activeView"
        />
      </KeepAlive>
      <section v-if="!viewComponent" class="flex h-full flex-col gap-3">
        <!-- Page title on its own row, aligned left -->
        <h1 class="shrink-0 text-[clamp(14px,1.6vw,16px)] font-semibold">
          {{ activeViewLabel }}
        </h1>
        <div class="flex flex-1 items-center justify-center">
          <div class="text-muted-foreground animate-in fade-in text-sm" :key="activeView">
            {{ activeViewLabel }} · 即将上线
          </div>
        </div>
      </section>
    </main>
    <SettingsModal v-if="showSettings" @close="showSettings = false" />
    <ToastContainer />
  </div>
</template>
