<script setup lang="ts">
import { Loader2, OctagonX, X } from "lucide-vue-next"
import { computed } from "vue"
import type { BuildProject } from "@/lib/build"

const props = defineProps<{
  project: BuildProject
  /** True while this project's build is running. */
  building?: boolean
}>()

const emit = defineEmits<{
  delete: []
  stop: []
  contextmenu: [event: MouseEvent]
}>()

// Source tags: configured project of the config area or a disk directory.
const sourceMeta = computed(() =>
  props.project.source === "config"
    ? {
        label: "配置项目",
        class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-500",
      }
    : {
        label: "磁盘项目",
        class: "bg-sky-500/15 text-sky-600 dark:text-sky-500",
      }
)
</script>

<template>
  <div
    class="bg-background/60 relative flex w-full flex-col gap-1.5 rounded-lg border border-border/60 px-2.5 py-2"
    @click.stop
    @contextmenu.prevent.stop="emit('contextmenu', $event)"
  >
    <div class="flex items-center gap-2 pr-6">
      <p
        class="min-w-0 flex-1 truncate text-[clamp(11px,1.2vw,12px)] font-semibold"
        :title="project.name"
      >
        {{ project.name }}
      </p>
      <!-- Source tag -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="sourceMeta.class"
      >
        {{ sourceMeta.label }}
      </span>
      <!-- Building state: spinner + stop action -->
      <template v-if="building">
        <Loader2
          class="text-muted-foreground size-3.5 shrink-0 animate-spin"
          aria-label="构建中"
          title="构建中…"
        />
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded transition-colors duration-200 focus-visible:outline-none"
          aria-label="停止构建"
          title="停止构建（结束构建进程及其子进程）"
          @click.stop="emit('stop')"
        >
          <OctagonX class="size-3.5" />
        </button>
      </template>
    </div>
    <!-- Remove from queue: files are never touched -->
    <button
      type="button"
      class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 absolute right-1.5 top-1.5 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors duration-200 focus-visible:outline-none"
      aria-label="从队列删除卡片"
      title="从队列删除卡片（项目文件不受影响）"
      @click="emit('delete')"
    >
      <X class="size-3" />
    </button>
    <p
      v-if="project.packageName"
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.packageName"
    >
      {{ project.packageName }}
    </p>
    <p
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.rootPath"
    >
      {{ project.rootPath }}
    </p>
  </div>
</template>
