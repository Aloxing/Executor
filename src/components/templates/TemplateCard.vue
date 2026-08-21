<script setup lang="ts">
import { Pencil, Trash2 } from "lucide-vue-next"
import type { TemplateInfo } from "@/lib/templates"

const props = defineProps<{
  template: TemplateInfo
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
}>()

const emit = defineEmits<{
  edit: []
  delete: []
  "toggle-select": []
  "import-code": []
  "import-parameter": []
  contextmenu: [event: MouseEvent]
}>()

function onClick() {
  if (props.selectMode) emit("toggle-select")
}
</script>

<template>
  <div
    class="flex w-full items-center gap-4 rounded-xl border px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
    :class="[
      selectMode && 'cursor-pointer',
      selected
        ? 'border-primary/40 bg-primary/[0.06]'
        : 'border-transparent bg-muted/40 hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]',
    ]"
    @click="onClick"
    @contextmenu.prevent="emit('contextmenu', $event)"
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
    <!-- Left: name + type badge -->
    <div class="flex shrink-0 items-center gap-2">
      <p class="max-w-[160px] truncate text-[clamp(12px,1.3vw,13px)] font-semibold">
        {{ template.name }}
      </p>
      <span
        class="bg-primary/10 text-primary shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
      >
        {{ template.templateType }}
      </span>
    </div>
    <!-- Middle: description -->
    <p
      class="text-muted-foreground min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)]"
      :title="template.description"
    >
      {{ template.description || "暂无介绍" }}
    </p>
    <!-- Times -->
    <div
      class="text-muted-foreground flex shrink-0 flex-col gap-0.5 text-[clamp(9px,1vw,10px)]"
    >
      <span>创建：{{ template.createdAt }}</span>
      <span>修改：{{ template.updatedAt }}</span>
    </div>
    <!-- Actions (hidden in batch selection mode) -->
    <div v-if="!selectMode" class="flex shrink-0 items-center gap-2">
      <button
        type="button"
        class="hover:bg-muted inline-flex h-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-2.5 text-[clamp(10px,1.1vw,11px)] font-medium transition-colors duration-200 focus-visible:outline-none"
        @click="emit('import-code')"
      >
        {{ template.codeImported ? "修改代码模板" : "导入代码模板" }}
      </button>
      <button
        type="button"
        class="hover:bg-muted inline-flex h-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-2.5 text-[clamp(10px,1.1vw,11px)] font-medium transition-colors duration-200 focus-visible:outline-none"
        @click="emit('import-parameter')"
      >
        {{ template.parameterImported ? "修改参数模板" : "导入参数模板" }}
      </button>
      <button
        type="button"
        class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
        aria-label="修改模板"
        title="修改模板"
        @click="emit('edit')"
      >
        <Pencil class="size-3" />
      </button>
      <button
        type="button"
        class="text-destructive hover:bg-destructive/10 inline-flex size-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
        aria-label="删除模板"
        title="删除模板"
        @click="emit('delete')"
      >
        <Trash2 class="size-3" />
      </button>
    </div>
  </div>
</template>
