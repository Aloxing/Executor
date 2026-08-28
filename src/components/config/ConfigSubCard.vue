<script setup lang="ts">
import { X } from "lucide-vue-next"
import { computed } from "vue"
import type { ConfigProject } from "@/lib/config"

const props = defineProps<{
  project: ConfigProject
}>()

const emit = defineEmits<{
  delete: []
  contextmenu: [event: MouseEvent]
}>()

// Template-selected tags, mirroring the import area's status tag style.
const statusMeta = computed(() =>
  props.project.templateName
    ? {
        label: "已选择模板",
        class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-500",
      }
    : { label: "未选择模板", class: "bg-muted text-muted-foreground" }
)

// Recorded tags: whether the contents were copied into the config area.
const recordedMeta = computed(() =>
  props.project.recorded
    ? {
        label: "已记录",
        class: "bg-sky-500/15 text-sky-600 dark:text-sky-500",
      }
    : { label: "未记录", class: "bg-muted text-muted-foreground" }
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
      <!-- Config status tag -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="statusMeta.class"
      >
        {{ statusMeta.label }}
      </span>
      <!-- Recorded tag -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="recordedMeta.class"
      >
        {{ recordedMeta.label }}
      </span>
    </div>
    <!-- Remove from queue: X pinned to the card's top-right corner; themed
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
      class="text-muted-foreground truncate text-[clamp(9px,1vw,10px)]"
      :title="project.startedAt ? `配置时间：${project.startedAt}` : '尚未开始配置'"
    >
      {{
        project.startedAt
          ? `配置时间：${project.startedAt}`
          : "配置时间：尚未开始"
      }}
    </p>
  </div>
</template>
