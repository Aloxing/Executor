<script setup lang="ts">
import { ChevronDown, Trash2, X } from "lucide-vue-next"
import { computed, ref } from "vue"
import type { OpRecord } from "@/lib/records"

const props = defineProps<{
  record: OpRecord
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
}>()

const emit = defineEmits<{
  delete: []
  "delete-item": [index: number]
  "toggle-select": []
}>()

// Collapsible sub-record list (伸缩模式).
const expanded = ref(false)

function onClick() {
  if (props.selectMode) {
    emit("toggle-select")
    return
  }
  if (props.record.items.length) expanded.value = !expanded.value
}

// Page tag: name + color per source page.
const pageMeta = computed(() => {
  switch (props.record.page) {
    case "import":
      return { label: "导入区", class: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" }
    case "config":
      return { label: "配置区", class: "bg-sky-500/10 text-sky-600 dark:text-sky-400" }
    case "build":
      return { label: "构建区", class: "bg-amber-500/10 text-amber-600 dark:text-amber-400" }
    case "output":
      return { label: "产出区", class: "bg-purple-500/10 text-purple-600 dark:text-purple-400" }
    default:
      return { label: props.record.page, class: "bg-muted text-muted-foreground" }
  }
})

// Operation tag: 新增 green / 删除 red / 修改 blue.
const actionMeta = computed(() => {
  switch (props.record.action) {
    case "add":
      return { label: "新增", class: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" }
    case "delete":
      return { label: "删除", class: "bg-red-500/10 text-red-600 dark:text-red-400" }
    case "modify":
      return { label: "修改", class: "bg-sky-500/10 text-sky-600 dark:text-sky-400" }
    default:
      return { label: props.record.action, class: "bg-muted text-muted-foreground" }
  }
})
</script>

<template>
  <div
    class="flex w-full flex-col rounded-xl border transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
    :class="[
      selectMode && 'cursor-pointer',
      selected
        ? 'border-primary/40 bg-primary/[0.06]'
        : 'border-transparent bg-muted/40 hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]',
    ]"
  >
    <!-- Summary row: same layout language as the template cards -->
    <div
      class="flex w-full items-center gap-3 px-3 py-2.5"
      :class="record.items.length || selectMode ? 'cursor-pointer' : ''"
      @click="onClick"
    >
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
      <!-- Tags: page + action -->
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="pageMeta.class"
      >
        {{ pageMeta.label }}
      </span>
      <span
        class="shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        :class="actionMeta.class"
      >
        {{ actionMeta.label }}
      </span>
      <!-- Title -->
      <p class="shrink-0 text-[clamp(12px,1.3vw,13px)] font-semibold">
        {{ record.title }}
      </p>
      <!-- Detail -->
      <p
        class="text-muted-foreground min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)]"
        :title="record.detail"
      >
        {{ record.detail }}
      </p>
      <!-- Sub-record count -->
      <span
        v-if="record.items.length"
        class="text-muted-foreground shrink-0 text-[clamp(9px,1vw,10px)]"
      >
        {{ record.items.length }} 条子记录
      </span>
      <!-- Time -->
      <div
        class="text-muted-foreground flex shrink-0 items-center gap-3 text-[clamp(9px,1vw,10px)]"
      >
        <span>{{ record.createdAt }}</span>
      </div>
      <!-- Actions (hidden in batch selection mode) -->
      <div v-if="!selectMode" class="flex shrink-0 items-center gap-2">
        <button
          v-if="record.items.length"
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          :aria-label="expanded ? '收起子记录' : '展开子记录'"
          :title="expanded ? '收起' : '展开'"
          @click.stop="expanded = !expanded"
        >
          <ChevronDown
            class="size-3 transition-transform duration-200"
            :class="expanded && 'rotate-180'"
          />
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          aria-label="删除记录"
          title="删除该记录卡片"
          @click.stop="emit('delete')"
        >
          <Trash2 class="size-3" />
        </button>
      </div>
    </div>
    <!-- Expanded sub records: one deletable row per affected entry -->
    <div
      v-if="expanded && !selectMode && record.items.length"
      class="flex flex-col gap-1 border-t border-border/60 px-3 py-2"
    >
      <div
        v-for="(item, index) in record.items"
        :key="`${record.uuid}-${index}`"
        class="bg-background/60 flex items-center gap-2 rounded-lg border border-border/60 px-2.5 py-1.5"
      >
        <span
          class="text-muted-foreground shrink-0 font-mono text-[clamp(9px,1vw,10px)]"
        >
          {{ index + 1 }}.
        </span>
        <p
          class="min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)] font-medium"
          :title="item"
        >
          {{ item }}
        </p>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="删除子记录"
          title="删除该子记录"
          @click.stop="emit('delete-item', index)"
        >
          <X class="size-3" />
        </button>
      </div>
    </div>
  </div>
</template>
