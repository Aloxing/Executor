<script setup lang="ts">
import { ChevronRight, Hammer, Loader2 } from "lucide-vue-next"
import { ref } from "vue"
import BuildSubCard from "./BuildSubCard.vue"
import type { BuildProject, BuildQueue } from "@/lib/build"

const props = defineProps<{
  queue: BuildQueue
  /** Uuid of the project currently building inside this queue, if any. */
  buildingUuid?: string
  /** True while a build-all of this queue is running. */
  building?: boolean
}>()

const emit = defineEmits<{
  "delete-project": [project: BuildProject]
  "stop-project": [project: BuildProject]
  contextmenu: [event: MouseEvent]
  // Sub cards carry their own context menu; forwarded without bubbling
  // into the queue card's menu.
  "project-contextmenu": [project: BuildProject, event: MouseEvent]
}>()

// Click the card to expand/collapse the project sub cards.
const expanded = ref(false)

function onClick() {
  if (props.building) return
  expanded.value = !expanded.value
}
</script>

<template>
  <div
    class="flex w-full cursor-pointer select-none flex-col gap-1.5 rounded-xl border border-transparent bg-muted/40 px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]"
    @click="onClick"
    @contextmenu.prevent="emit('contextmenu', $event)"
  >
    <!-- Top-left: type badge + queue name -->
    <div class="flex items-center gap-2">
      <span
        class="bg-sky-500/15 text-sky-600 dark:text-sky-500 inline-flex size-6 shrink-0 items-center justify-center rounded-md"
        aria-hidden="true"
        title="构建队列"
      >
        <Hammer class="size-3.5" />
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
      <!-- Loading indicator while a build is running -->
      <Loader2
        v-if="building"
        class="text-muted-foreground size-3.5 shrink-0 animate-spin"
        aria-label="构建中"
        title="队列构建中…"
      />
      <ChevronRight
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
    <!-- Project sub cards -->
    <div v-if="expanded" class="flex flex-col gap-1.5 pt-1">
      <BuildSubCard
        v-for="project in queue.projects"
        :key="project.uuid"
        :project="project"
        :building="buildingUuid === project.uuid"
        @delete="emit('delete-project', project)"
        @stop="emit('stop-project', project)"
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
