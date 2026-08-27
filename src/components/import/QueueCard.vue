<script setup lang="ts">
import { ChevronRight, Download, Loader2 } from "lucide-vue-next"
import { computed, ref } from "vue"
import AndroidSubCard from "./AndroidSubCard.vue"
import type { AndroidProject } from "@/lib/android"
import type { ImportQueue } from "@/lib/queues"

const props = defineProps<{
  queue: ImportQueue
  /** Android projects attached to this queue. */
  projects: AndroidProject[]
  /** True while the queue's import action is running. */
  importing?: boolean
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
}>()

const emit = defineEmits<{
  import: []
  "delete-project": [project: AndroidProject]
  "toggle-select": []
  contextmenu: [event: MouseEvent]
}>()

// Click the card to expand/collapse the Android project sub cards, or to
// toggle the batch selection when in select mode.
const expanded = ref(false)

// The import action is only available while the queue still holds projects
// that have not been imported yet.
const hasPending = computed(() =>
  props.projects.some((p) => p.importStatus === "pending")
)

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
      <!-- Lucide dropped brand icons, so the Android robot is inlined. -->
      <span
        class="bg-[#3DDC84]/15 text-[#3DDC84] inline-flex size-6 shrink-0 items-center justify-center rounded-md"
        aria-hidden="true"
        title="Android"
      >
        <svg viewBox="0 0 24 24" fill="currentColor" class="size-3.5">
          <path
            d="M17.523 15.3414c-.5511 0-.9993-.4486-.9993-.9997s.4483-.9993.9993-.9993c.5511 0 .9993.4483.9993.9993.0001.5511-.4482.9997-.9993.9997m-11.046 0c-.5511 0-.9993-.4486-.9993-.9997s.4482-.9993.9993-.9993c.5511 0 .9993.4483.9993.9993 0 .5511-.4483.9997-.9993.9997m11.4045-6.02l1.9973-3.4592a.416.416 0 0 0-.1521-.5676.416.416 0 0 0-.5676.1521l-2.0223 3.503C15.5902 8.2439 13.8533 7.8508 12 7.8508s-3.5902.3931-5.1368 1.0989L4.8409 5.4467a.4161.4161 0 0 0-.5677-.1521.4157.4157 0 0 0-.1521.5676l1.9973 3.4592C2.6889 11.1867.3432 14.6589 0 18.761h24c-.3435-4.1021-2.6892-7.5743-6.1185-9.4396"
          />
        </svg>
      </span>
      <p
        class="min-w-0 flex-1 truncate text-[clamp(12px,1.3vw,13px)] font-semibold"
        :title="queue.name"
      >
        {{ queue.name }}
      </p>
      <!-- Actions (hidden in batch selection mode) -->
      <template v-if="!selectMode">
        <!-- Run import (only this action copies files into package folders) -->
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="importing || !hasPending"
          :title="importing ? '导入中…' : '按包名导入该队列下未导入的 Android 项目'"
          aria-label="进行导入"
          @click.stop="emit('import')"
        >
          <Loader2 v-if="importing" class="size-3 animate-spin" />
          <Download v-else class="size-3" />
        </button>
        <ChevronRight
          class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-200"
          :class="expanded ? 'rotate-90' : ''"
        />
      </template>
    </div>
    <!-- Creation time -->
    <p
      class="text-muted-foreground truncate text-[clamp(9px,1vw,10px)]"
      :title="queue.createdAt"
    >
      创建：{{ queue.createdAt }}
    </p>
    <!-- Android project sub cards -->
    <div v-if="expanded && !selectMode" class="flex flex-col gap-1.5 pt-1">
      <AndroidSubCard
        v-for="project in projects"
        :key="project.packageName"
        :project="project"
        @delete="emit('delete-project', project)"
      />
      <p
        v-if="!projects.length"
        class="text-muted-foreground px-1 py-1 text-center text-[clamp(9px,1vw,10px)]"
      >
        暂无 Android 项目，右键卡片添加
      </p>
    </div>
  </div>
</template>
