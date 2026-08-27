<script setup lang="ts">
import { X } from "lucide-vue-next"
import { computed } from "vue"
import type { AndroidProject } from "@/lib/android"

const props = defineProps<{
  project: AndroidProject
}>()

const emit = defineEmits<{
  delete: []
}>()

// Three import states with distinct colors, persisted in android.json.
const statusMeta = computed(() => {
  switch (props.project.importStatus) {
    case "importing":
      return {
        label: "导入中",
        class: "bg-amber-500/15 text-amber-600 dark:text-amber-500",
      }
    case "imported":
      return {
        label: "已导入",
        class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-500",
      }
    default:
      return { label: "未导入", class: "bg-muted text-muted-foreground" }
  }
})
</script>

<template>
  <div
    class="bg-background/60 relative flex w-full flex-col gap-1.5 rounded-lg border border-border/60 px-2.5 py-2"
    @click.stop
  >
    <div class="flex items-center gap-2 pr-6">
      <p
        class="min-w-0 flex-1 truncate text-[clamp(11px,1.2vw,12px)] font-semibold"
        :title="project.appName"
      >
        {{ project.appName }}
      </p>
      <!-- Import status tag -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="statusMeta.class"
      >
        {{ statusMeta.label }}
      </span>
    </div>
    <!-- Detach from queue: X pinned to the card's top-right corner; themed
         by default, turns red only on hover. -->
    <button
      type="button"
      class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 absolute right-1.5 top-1.5 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors duration-200 focus-visible:outline-none"
      aria-label="从队列删除卡片"
      title="从队列删除卡片"
      @click="emit('delete')"
    >
      <X class="size-3" />
    </button>
    <p
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.packageName"
    >
      {{ project.packageName }}
    </p>
  </div>
</template>
