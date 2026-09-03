<script setup lang="ts">
import { FolderDown, LayoutTemplate, MapPin, RefreshCw } from "lucide-vue-next"
import { computed } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import type { ConfigProject } from "@/lib/config"

const props = defineProps<{
  x: number
  y: number
  project: ConfigProject
}>()

const emit = defineEmits<{
  close: []
  "pick-template": []
  record: []
  locate: []
  "reload-project": []
}>()

// Keep the menu inside the viewport (approximate menu size 220x170).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 230))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 180))}px`,
}))

// Closing is driven by the central shortcut system (Esc by default).
useShortcut("close", () => emit("close"))
</script>

<template>
  <div
    class="fixed inset-0 z-50"
    @click="emit('close')"
    @contextmenu.prevent="emit('close')"
  >
    <div
      role="menu"
      class="bg-popover text-popover-foreground absolute min-w-[220px] rounded-lg border border-border p-1 shadow-md"
      :style="position"
      @click.stop
    >
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="选择模板并保存参数，可直接开始配置"
        @click="emit('pick-template')"
      >
        <LayoutTemplate class="size-3.5 shrink-0" />
        选择配置模板
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="导入区的项目从导入区目录复制，磁盘的项目从磁盘目录复制"
        @click="emit('record')"
      >
        <FolderDown class="size-3.5 shrink-0" />
        记录项目
      </button>
      <!-- Recorded imported projects only: rebuild the config copy from
           the import area while the parameter JSON stays untouched. -->
      <button
        v-if="project.source === 'imported' && project.recorded"
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="删除配置目录下已复制的项目文件并从导入区重新复制；参数 JSON 与模板选择保留"
        @click="emit('reload-project')"
      >
        <RefreshCw class="size-3.5 shrink-0" />
        重载项目（保留参数）
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="在资源管理器中打开项目目录"
        @click="emit('locate')"
      >
        <MapPin class="size-3.5 shrink-0" />
        定位项目
      </button>
    </div>
  </div>
</template>
