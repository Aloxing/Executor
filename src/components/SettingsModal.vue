<script setup lang="ts">
import { Hammer, HardDrive, Info, Palette, Settings, X } from "lucide-vue-next"
import { ref, type Component } from "vue"
import GeneralSettings from "./settings/GeneralSettings.vue"
import AppearanceSettings from "./settings/AppearanceSettings.vue"
import CompileSettings from "./settings/CompileSettings.vue"
import StorageSettings from "./settings/StorageSettings.vue"
import { useShortcut } from "@/lib/shortcuts"

const emit = defineEmits<{ close: [] }>()

interface SettingsTab {
  key: string
  label: string
  icon: Component
}

// New tabs only need to be added here; the nav renders icon + label
// automatically.
const tabs: SettingsTab[] = [
  { key: "general", label: "通用", icon: Settings },
  { key: "appearance", label: "外观", icon: Palette },
  { key: "compile", label: "编译", icon: Hammer },
  { key: "storage", label: "存储", icon: HardDrive },
  { key: "about", label: "关于", icon: Info },
]

const activeTab = ref(tabs[0])

function selectTab(tab: SettingsTab) {
  activeTab.value = tab
}

// Closing is driven by the central shortcut system (Esc by default).
useShortcut("close", () => emit("close"))
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-[3%]">
    <!-- Backdrop: plain translucent fill — backdrop-blur is very
         expensive here because the WebView runs with GPU disabled
         (software compositing), which made opening laggy -->
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-modal-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex h-[min(86%,760px)] w-[min(90%,1000px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(12px,2vw,16px)] py-[clamp(8px,1.4vh,12px)]"
      >
        <h2
          id="settings-modal-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          设置
        </h2>
        <button
          type="button"
          class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
          aria-label="关闭设置"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div class="flex min-h-0 flex-1">
        <nav
          class="flex w-[clamp(110px,16vw,160px)] shrink-0 flex-col gap-1 border-r border-border p-[clamp(8px,1.4vh,12px)]"
          aria-label="设置导航"
        >
          <button
            v-for="tab in tabs"
            :key="tab.key"
            type="button"
            class="inline-flex h-[clamp(26px,4vh,32px)] cursor-pointer items-center gap-1.5 rounded-md border-none px-[clamp(8px,1vw,10px)] text-left text-[clamp(11px,1.25vw,12px)] font-medium transition-colors focus-visible:outline-none"
            :class="
              activeTab.key === tab.key
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-accent/60 hover:text-accent-foreground bg-transparent'
            "
            :aria-current="activeTab.key === tab.key ? 'page' : undefined"
            @click="selectTab(tab)"
          >
            <component :is="tab.icon" class="size-3.5 shrink-0" />
            <span class="truncate">{{ tab.label }}</span>
          </button>
        </nav>
        <!-- Tabs are kept alive so switching never rebuilds them -->
        <div class="flex min-w-0 flex-1 flex-col overflow-auto p-[17px]">
          <KeepAlive>
            <GeneralSettings v-if="activeTab.key === 'general'" />
            <AppearanceSettings v-else-if="activeTab.key === 'appearance'" />
            <CompileSettings v-else-if="activeTab.key === 'compile'" />
            <StorageSettings v-else-if="activeTab.key === 'storage'" />
            <div
              v-else
              class="flex flex-1 items-center justify-center"
            >
              <p class="text-muted-foreground text-xs">
                {{ activeTab.label }} · 即将上线
              </p>
            </div>
          </KeepAlive>
        </div>
      </div>
    </div>
  </div>
</template>
