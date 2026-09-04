<script setup lang="ts">
import { Eraser, FolderOpen, PackageOpen, Play, Trash2 } from "lucide-vue-next"
import { computed } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import type { BuildQueue } from "@/lib/build"

const props = defineProps<{
  x: number
  y: number
  queue: BuildQueue
}>()

const emit = defineEmits<{
  close: []
  "pick-config": []
  "pick-disk": []
  /** Opens the build-mode dialog (command + serial/parallel). */
  "build-all-open": []
  /** Removes every project card of the queue, keeping the queue itself. */
  "clear-queue": []
  "delete-queue": []
}>()

// Keep the menu inside the viewport (approximate menu size 230x250).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 240))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 260))}px`,
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
        title="从配置区选择已完善配置的项目，仅记录地址"
        @click="emit('pick-config')"
      >
        <PackageOpen class="size-3.5 shrink-0" />
        添加项目：从配置区项目
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="从磁盘选择项目目录，仅记录地址不复制文件"
        @click="emit('pick-disk')"
      >
        <FolderOpen class="size-3.5 shrink-0" />
        添加项目：从磁盘目录
      </button>
      <div class="bg-border mx-1 my-1 h-px" aria-hidden="true" />
      <!-- Build-all: the mode dialog picks the command and serial/parallel -->
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="为队列下全部项目构建：选择构建命令与串行/并行方式"
        @click="emit('build-all-open')"
      >
        <Play class="size-3.5 shrink-0" />
        全部构建…
      </button>
      <div class="bg-border mx-1 my-1 h-px" aria-hidden="true" />
      <button
        type="button"
        role="menuitem"
        class="text-destructive hover:bg-destructive/10 flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="移除队列下的全部项目卡片，队列保留，项目文件不受影响"
        @click="emit('clear-queue')"
      >
        <Eraser class="size-3.5 shrink-0" />
        清空队列
      </button>
      <button
        type="button"
        role="menuitem"
        class="text-destructive hover:bg-destructive/10 flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="仅删除队列与卡片记录，项目文件不受影响"
        @click="emit('delete-queue')"
      >
        <Trash2 class="size-3.5 shrink-0" />
        删除队列
      </button>
    </div>
  </div>
</template>
