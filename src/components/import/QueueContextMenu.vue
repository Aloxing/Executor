<script setup lang="ts">
import { AppWindow, Trash2 } from "lucide-vue-next"
import { computed, onMounted, onUnmounted } from "vue"
import type { ImportQueue } from "@/lib/queues"

const props = defineProps<{
  x: number
  y: number
  queue: ImportQueue
}>()

const emit = defineEmits<{
  close: []
  "add-android": []
  "delete-queue": []
}>()

// Keep the menu inside the viewport (approximate menu size 180x100).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 190))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 110))}px`,
}))

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown)
})

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})
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
