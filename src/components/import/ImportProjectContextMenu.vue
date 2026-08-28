<script setup lang="ts">
import { FolderDown } from "lucide-vue-next"
import { computed, onMounted, onUnmounted } from "vue"
import type { AndroidProject } from "@/lib/android"

const props = defineProps<{
  x: number
  y: number
  project: AndroidProject
}>()

const emit = defineEmits<{
  close: []
  record: []
}>()

// Keep the menu inside the viewport (approximate menu size 180x50).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 190))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 60))}px`,
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
        title="按包名将该项目复制到导入目录"
        @click="emit('record')"
      >
        <FolderDown class="size-3.5 shrink-0" />
        记录项目
      </button>
    </div>
  </div>
</template>
