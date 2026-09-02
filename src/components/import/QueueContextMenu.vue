<script setup lang="ts">
import { AppWindow, FolderDown, Trash2 } from "lucide-vue-next"
import { computed } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import type { ImportQueue } from "@/lib/queues"

const props = defineProps<{
  x: number
  y: number
  queue: ImportQueue
}>()

const emit = defineEmits<{
  close: []
  "add-android": []
  "record-all": []
  "delete-queue": []
}>()

// Keep the menu inside the viewport (approximate menu size 180x130).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 190))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 140))}px`,
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
      class="bg-popover text-popover-foreground absolute min-w-[180px] rounded-lg border border-border p-1 shadow-md"
      :style="position"
      @click.stop
    >
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('add-android')"
      >
        <AppWindow class="size-3.5 shrink-0" />
        添加 Android 项目
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="按包名将该队列下全部项目复制到导入目录"
        @click="emit('record-all')"
      >
        <FolderDown class="size-3.5 shrink-0" />
        记录全部项目
      </button>
      <div class="bg-border mx-1 my-1 h-px" aria-hidden="true" />
      <button
        type="button"
        role="menuitem"
        class="text-destructive hover:bg-destructive/10 flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('delete-queue')"
      >
        <Trash2 class="size-3.5 shrink-0" />
        删除队列
      </button>
    </div>
  </div>
</template>
