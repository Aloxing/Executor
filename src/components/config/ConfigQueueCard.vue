<script setup lang="ts">
import { ChevronRight, Cog, Loader2 } from "lucide-vue-next"
import { ref } from "vue"
import ConfigSubCard from "./ConfigSubCard.vue"
import type { ConfigProject, ConfigQueue } from "@/lib/config"

const props = defineProps<{
  queue: ConfigQueue
  /** True while the queue is receiving projects (copy in progress). */
  adding?: boolean
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
}>()

const emit = defineEmits<{
  "delete-project": [project: ConfigProject]
  contextmenu: [event: MouseEvent]
  // Sub cards carry their own context menu; forwarded without bubbling
  // into the queue card's menu.
  "project-contextmenu": [project: ConfigProject, event: MouseEvent]
  "toggle-select": []
}>()

// Click the card to expand/collapse the sub project cards, or to toggle
// the batch selection when in select mode.
const expanded = ref(false)

function onClick() {
  if (props.selectMode) {
    emit("toggle-select")
    return
  }
  expanded.value = !expanded.value
}
</script>

<template>
  <div
    class="flex w-full cursor-pointer select-none flex-col gap-1.5 rounded-xl border px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
    :class="[
      selected
        ? 'border-primary/40 bg-primary/[0.06]'
        : 'border-transparent bg-muted/40 hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]',
    ]"
    @click="onClick"
    @contextmenu.prevent="emit('contextmenu', $event)"
  >
    <!-- Top-left: type badge + queue name -->
    <div class="flex items-center gap-2">
      <!-- Selection checkbox (batch mode only) -->
      <span
        v-if="selectMode"
        aria-hidden="true"
        class="flex size-4 shrink-0 items-center justify-center rounded-[4px] border transition-all duration-200 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
        :class="selected ? 'border-primary bg-primary' : 'border-input bg-transparent'"
      >
        <svg
          v-if="selected"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="3.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-primary-foreground size-[10px]"
        >
          <path d="M20 6 9 17l-5-5" />
        </svg>
      </span>
      <span
        class="bg-sky-500/15 text-sky-600 dark:text-sky-500 inline-flex size-6 shrink-0 items-center justify-center rounded-md"
        aria-hidden="true"
        title="配置队列"
      >
        <Cog class="size-3.5" />
      </span>
      <p
        class="min-w-0 flex-1 truncate text-[clamp(12px,1.3vw,13px)] font-semibold"
        :title="queue.name"
      >
        {{ queue.name }}
      </p>
      <span
        class="text-muted-foreground shrink-0 text-[clamp(9px,1vw,10px)]"
      >
        {{ queue.projects.length }} 个项目
      </span>
      <!-- Loading indicator while projects are being copied in -->
      <Loader2
        v-if="adding"
        class="text-muted-foreground size-3.5 shrink-0 animate-spin"
        aria-label="添加中"
        title="项目添加中…"
      />
      <ChevronRight
        v-if="!selectMode"
        class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-200"
        :class="expanded ? 'rotate-90' : ''"
      />
    </div>
    <!-- Creation time -->
    <p
      class="text-muted-foreground truncate text-[clamp(9px,1vw,10px)]"
      :title="queue.createdAt"
    >
      创建：{{ queue.createdAt }}
    </p>
    <!-- Sub project cards -->
    <div v-if="expanded && !selectMode" class="flex flex-col gap-1.5 pt-1">
      <ConfigSubCard
        v-for="project in queue.projects"
        :key="project.uuid"
        :project="project"
        @delete="emit('delete-project', project)"
        @contextmenu="emit('project-contextmenu', project, $event)"
      />
      <p
        v-if="!queue.projects.length"
        class="text-muted-foreground px-1 py-1 text-center text-[clamp(9px,1vw,10px)]"
      >
        暂无项目，右键卡片添加
      </p>
    </div>
  </div>
</template>
