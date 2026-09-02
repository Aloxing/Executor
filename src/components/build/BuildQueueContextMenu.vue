<script setup lang="ts">
import { FolderOpen, PackageOpen, Play } from "lucide-vue-next"
import { computed } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import { buildCommands, type BuildQueue } from "@/lib/build"

const props = defineProps<{
  x: number
  y: number
  queue: BuildQueue
}>()

const emit = defineEmits<{
  close: []
  "pick-config": []
  "pick-disk": []
  /** Runs the chosen gradle flow for every project of the queue. */
  "build-all": [args: string[]]
}>()

// Keep the menu inside the viewport (approximate menu size 230x210).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 240))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 220))}px`,
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
      class="bg-popover text-popover-foreground absolute min-w-[230px] rounded-lg border border-border p-1 shadow-md"
      :style="position"
      @click.stop
    >
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('pick-config')"
      >
        <PackageOpen class="size-3.5 shrink-0" />
        从已完善配置的项目构建
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('pick-disk')"
      >
        <FolderOpen class="size-3.5 shrink-0" />
        从磁盘中项目选择构建
      </button>
      <div class="bg-border mx-1 my-1 h-px" aria-hidden="true" />
      <!-- Build-all commands: gradle wrapper first, then gradlew <args> -->
      <button
        v-for="command in buildCommands"
        :key="command.label"
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        :title="`为队列下全部项目执行 gradle wrapper → gradlew ${command.label}`"
        @click="emit('build-all', command.args)"
      >
        <Play class="size-3.5 shrink-0" />
        全部构建：{{ command.label }}
      </button>
    </div>
  </div>
</template>
