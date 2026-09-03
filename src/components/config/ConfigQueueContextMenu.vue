<script setup lang="ts">
import { FolderDown, FolderOpen, PackageOpen, Trash2, Wand2 } from "lucide-vue-next"
import { computed } from "vue"
import { useShortcut } from "@/lib/shortcuts"
import type { ConfigQueue } from "@/lib/config"

const props = defineProps<{
  x: number
  y: number
  queue: ConfigQueue
}>()

const emit = defineEmits<{
  close: []
  "pick-imported": []
  "pick-disk": []
  "record-all": []
  "delete-queue": []
  /** Automation pipeline: batch template + config + forward to build. */
  "batch-template": []
}>()

// Keep the menu inside the viewport (approximate menu size 200x200).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 210))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 210))}px`,
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
      class="bg-popover text-popover-foreground absolute min-w-[200px] rounded-lg border border-border p-1 shadow-md"
      :style="position"
      @click.stop
    >
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('pick-imported')"
      >
        <PackageOpen class="size-3.5 shrink-0" />
        添加项目：从已导入项目
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        @click="emit('pick-disk')"
      >
        <FolderOpen class="size-3.5 shrink-0" />
        添加项目：从磁盘目录
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="导入项目从导入区复制、磁盘项目从源目录复制到配置区"
        @click="emit('record-all')"
      >
        <FolderDown class="size-3.5 shrink-0" />
        记录全部项目
      </button>
      <button
        type="button"
        role="menuitem"
        class="hover:bg-accent hover:text-accent-foreground flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="自动化流水线：选一次模板应用到全部项目（保存模板→开始配置→完善配置），完成后自动转入构建区"
        @click="emit('batch-template')"
      >
        <Wand2 class="size-3.5 shrink-0" />
        批量模板配置
      </button>
      <div class="bg-border mx-1 my-1 h-px" aria-hidden="true" />
      <button
        type="button"
        role="menuitem"
        class="text-destructive hover:bg-destructive/10 flex w-full cursor-pointer items-center gap-2 rounded-md border-none bg-transparent px-2 py-1.5 text-left text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none"
        title="仅删除队列与卡片记录，磁盘上已复制的文件保留"
        @click="emit('delete-queue')"
      >
        <Trash2 class="size-3.5 shrink-0" />
        删除队列
      </button>
    </div>
  </div>
</template>
