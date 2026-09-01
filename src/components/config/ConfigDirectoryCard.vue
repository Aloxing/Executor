<script setup lang="ts">
import { ChevronRight, Cog, Loader2, Pencil, Rocket, Trash2 } from "lucide-vue-next"
import { computed, ref } from "vue"
import ParameterCard from "./ParameterCard.vue"
import type { ConfigProject } from "@/lib/config"

const props = defineProps<{
  /** A configured (started) sub project. */
  project: ConfigProject
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
  /** True while the launch (kernel injection) action is running. */
  executing?: boolean
}>()

const emit = defineEmits<{
  edit: []
  execute: []
  delete: []
  "toggle-select": []
}>()

function onClick() {
  if (props.selectMode) {
    emit("toggle-select")
    return
  }
  // Click the card to expand/collapse the parameter card section.
  expanded.value = !expanded.value
  // First expand mounts the ParameterCard; later collapses only hide it.
  if (expanded.value) paramLoaded.value = true
}

const expanded = ref(false)
// Once expanded for the first time the ParameterCard stays mounted (only
// hidden on collapse), so unsaved edits survive collapsing the card.
const paramLoaded = ref(false)

// Imported / non-imported source tag, mirroring the import area's status
// tag style.
const sourceMeta = computed(() =>
  props.project.source === "imported"
    ? {
        label: "导入项目",
        class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-500",
      }
    : {
        label: "非导入项目",
        class: "bg-sky-500/15 text-sky-600 dark:text-sky-500",
      }
)
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
  >
    <!-- Top-left: type badge + project name; right: template tag -->
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
        title="配置项目"
      >
        <Cog class="size-3.5" />
      </span>
      <p
        class="min-w-0 flex-1 truncate text-[clamp(12px,1.3vw,13px)] font-semibold"
        :title="project.name"
      >
        {{ project.name }}
      </p>
      <!-- Imported / non-imported source tag -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="sourceMeta.class"
      >
        {{ sourceMeta.label }}
      </span>
      <!-- Template name tag -->
      <span
        v-if="project.templateName"
        class="shrink-0 rounded-md bg-sky-500/15 px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium text-sky-600 dark:text-sky-500"
        :title="`配置模板：${project.templateName}`"
      >
        {{ project.templateName }}
      </span>
      <!-- Actions (hidden in batch selection mode) -->
      <template v-if="!selectMode">
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-muted/60 transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="executing"
          aria-label="启动"
          title="启动：完善配置并执行内核注入"
          @click.stop="emit('execute')"
        >
          <Loader2 v-if="executing" class="size-3 animate-spin" />
          <Rocket v-else class="size-3" />
        </button>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          aria-label="查看项目信息"
          title="查看项目信息"
          @click.stop="emit('edit')"
        >
          <Pencil class="size-3" />
        </button>
        <!-- Delete the project record and its copied config directory -->
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          aria-label="删除项目"
          title="删除项目与配置目录"
          @click.stop="emit('delete')"
        >
          <Trash2 class="size-3" />
        </button>
        <ChevronRight
          class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-200"
          :class="expanded ? 'rotate-90' : ''"
        />
      </template>
    </div>
    <!-- Package name -->
    <p
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.packageName || '—'"
    >
      {{ project.packageName || "—" }}
    </p>
    <!-- Project address: empty until the project is recorded -->
    <p
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.recorded ? project.rootPath : ''"
    >
      {{ project.recorded ? project.rootPath : "" }}
    </p>
    <!-- Config time -->
    <p
      class="text-muted-foreground truncate text-[clamp(8px,0.9vw,9px)]"
      :title="`配置时间：${project.startedAt}`"
    >
      配置时间：{{ project.startedAt }}
    </p>
    <!-- Parameter card section: mounted on first expand, then only
         hidden, so unsaved edits survive collapsing -->
    <div v-if="paramLoaded" v-show="expanded && !selectMode" class="flex flex-col gap-1.5 pt-1">
      <div
        class="bg-background/60 flex w-full flex-col gap-1.5 rounded-lg border border-border/60 px-2.5 py-2"
        @click.stop
      >
        <p class="text-[clamp(11px,1.2vw,12px)] font-semibold">参数卡片</p>
        <ParameterCard :project="project" />
      </div>
    </div>
  </div>
</template>
