<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue"
import AppTitleBar from "./components/AppTitleBar.vue"
import FloatingNavBar, { type NavKey } from "./components/FloatingNavBar.vue"
import SettingsModal from "./components/SettingsModal.vue"
import TemplatesView from "./components/templates/TemplatesView.vue"
import ToastContainer from "./components/ToastContainer.vue"
import { loadSettings, settings } from "./lib/settings"
import { applyThemeMode } from "./lib/theme"

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

onMounted(async () => {
  await nextTick()
  // Load persisted settings (close behavior etc.) before the window shows.
  await loadSettings()
  applyThemeMode(settings.themeMode)
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
      <TemplatesView v-if="activeView === 'templates'" />
      <section v-else class="flex h-full flex-col gap-3">
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
