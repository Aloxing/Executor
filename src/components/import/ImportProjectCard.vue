<script setup lang="ts">
import { Pencil, Trash2 } from "lucide-vue-next"
import { computed } from "vue"
import type { AndroidProject } from "@/lib/android"

const props = defineProps<{
  project: AndroidProject
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
}>()

const emit = defineEmits<{
  edit: []
  delete: []
  "toggle-select": []
  contextmenu: [event: MouseEvent]
}>()

function onClick() {
  if (props.selectMode) emit("toggle-select")
}

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
    class="flex w-full flex-col gap-1.5 rounded-xl border px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
    :class="[
      selectMode && 'cursor-pointer',
      selected
        ? 'border-primary/40 bg-primary/[0.06]'
        : 'border-transparent bg-muted/40 hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]',
    ]"
    @click="onClick"
    @contextmenu.prevent="emit('contextmenu', $event)"
  >
    <!-- Top-left: type badge + app name; right: status tag and actions -->
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
      <!-- Actions (hidden in batch selection mode) -->
      <template v-if="!selectMode">
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          aria-label="修改项目信息"
          title="修改项目信息"
          @click="emit('edit')"
        >
          <Pencil class="size-3" />
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          aria-label="删除项目"
          title="删除项目（含已导入的包名文件夹，不可恢复）"
          @click="emit('delete')"
        >
          <Trash2 class="size-3" />
        </button>
      </template>
    </div>
    <!-- Package name -->
    <p
      class="text-muted-foreground truncate font-mono text-[clamp(9px,1vw,10px)]"
      :title="project.packageName"
    >
      {{ project.packageName }}
    </p>
    <!-- Imported location (never the manually picked source path) -->
    <p
      class="text-muted-foreground truncate text-[clamp(9px,1vw,10px)]"
      :title="project.location || '尚未导入'"
    >
      {{ project.location || "尚未导入，导入后位于工作空间 import/package/ 下" }}
    </p>
    <!-- Times -->
    <div
      class="text-muted-foreground flex items-center justify-between text-[clamp(8px,0.9vw,9px)]"
    >
      <span>创建：{{ project.createdAt }}</span>
      <span>修改：{{ project.updatedAt }}</span>
    </div>
  </div>
</template>
