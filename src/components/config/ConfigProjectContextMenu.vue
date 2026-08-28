<script setup lang="ts">
import { FolderDown, LayoutTemplate } from "lucide-vue-next"
import { computed, onMounted, onUnmounted } from "vue"
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
}>()

// Keep the menu inside the viewport (approximate menu size 220x90).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 230))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 100))}px`,
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
      class="bg-popover text-popover-foreground absolute min-w-[220px] rounded-lg border border-border p-1 shadow-md"
      :style="position"
      @click.stop
    >
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('pick-template')"
      >
        <LayoutTemplate class="size-3.5 shrink-0" />
        选择配置模板后开始配置
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
    </div>
  </div>
</template>
