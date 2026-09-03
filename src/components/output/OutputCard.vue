<script setup lang="ts">
import { ChevronDown, Copy, FileBox, Loader2, SmartphoneNfc, Trash2 } from "lucide-vue-next"
import { ref } from "vue"
import type { OutputFile, OutputRecord } from "@/lib/output"

const props = defineProps<{
  record: OutputRecord
  /** Batch selection mode: card becomes a toggle, actions are hidden. */
  selectMode?: boolean
  selected?: boolean
  /** True when an adb device is connected; shows the apk install button. */
  canInstall?: boolean
  /** Path of the file currently installing (spinner on its row). */
  installingPath?: string
}>()

const emit = defineEmits<{
  delete: []
  "delete-file": [file: OutputFile]
  "copy-file": [file: OutputFile]
  "install-file": [file: OutputFile]
  "toggle-select": []
}>()

// Only apk artifacts can be installed onto a phone.
function isApk(name: string): boolean {
  return name.toLowerCase().endsWith(".apk")
}

// Collapsible file list (伸缩模式): the card expands to show every
// artifact with its own copy/delete actions.
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
    class="flex w-full flex-col rounded-xl border transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
    :class="[
      selectMode && 'cursor-pointer',
      selected
        ? 'border-primary/40 bg-primary/[0.06]'
        : 'border-transparent bg-muted/40 hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]',
    ]"
  >
    <!-- Summary row: same layout language as the template cards -->
    <div class="flex w-full cursor-pointer items-center gap-4 px-3 py-2.5" @click="onClick">
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
      <!-- Left: project name + template tag + build type -->
      <div class="flex shrink-0 items-center gap-2">
        <p
          class="max-w-[180px] truncate text-[clamp(12px,1.3vw,13px)] font-semibold"
          :title="record.projectName"
        >
          {{ record.projectName }}
        </p>
        <!-- Template name tag; direct disk builds carry no template -->
        <span
          v-if="record.templateName"
          class="bg-primary/10 text-primary shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
          :title="`模板：${record.templateName}`"
        >
          {{ record.templateName }}
        </span>
        <span
          v-else
          class="text-muted-foreground shrink-0 rounded-md bg-muted px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
        >
          无模板
        </span>
        <span
          class="shrink-0 rounded-md bg-muted px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium uppercase"
        >
          {{ record.buildType }}
        </span>
      </div>
      <!-- Middle: package name + file count -->
      <p
        class="text-muted-foreground min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)]"
        :title="record.rootPath"
      >
        {{ record.packageName ? `${record.packageName} · ` : "" }}{{
          record.files.length
        }}
        个产出文件
      </p>
      <!-- Record time -->
      <div
        class="text-muted-foreground flex shrink-0 items-center gap-3 text-[clamp(9px,1vw,10px)]"
      >
        <span>记录：{{ record.createdAt }}</span>
      </div>
      <!-- Actions (hidden in batch selection mode) -->
      <div v-if="!selectMode" class="flex shrink-0 items-center gap-2">
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
          :aria-label="expanded ? '收起文件列表' : '展开文件列表'"
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
          aria-label="删除产出卡片"
          title="删除卡片并真删除其产出文件"
          @click.stop="emit('delete')"
        >
          <Trash2 class="size-3" />
        </button>
      </div>
    </div>
    <!-- Expanded artifact list: one row per file with copy/delete -->
    <div
      v-if="expanded && !selectMode"
      class="flex flex-col gap-1 border-t border-border/60 px-3 py-2"
    >
      <div
        v-for="file in record.files"
        :key="file.path"
        class="bg-background/60 flex items-center gap-2 rounded-lg border border-border/60 px-2.5 py-1.5"
      >
        <FileBox class="text-muted-foreground size-3.5 shrink-0" />
        <p
          class="min-w-0 flex-1 truncate text-[clamp(10px,1.1vw,11px)] font-medium"
          :title="file.path"
        >
          {{ file.name }}
        </p>
        <button
          v-if="canInstall && isApk(file.name)"
          type="button"
          class="text-primary hover:bg-primary/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="installingPath === file.path"
          aria-label="安装到手机"
          title="adb install：直接安装该 apk 到已连接的手机（保留应用数据覆盖安装）"
          @click.stop="emit('install-file', file)"
        >
          <Loader2 v-if="installingPath === file.path" class="size-3 animate-spin" />
          <SmartphoneNfc v-else class="size-3" />
        </button>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="复制文件"
          title="复制文件到剪贴板（可在资源管理器中粘贴）"
          @click.stop="emit('copy-file', file)"
        >
          <Copy class="size-3" />
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="删除文件"
          title="真删除该产出文件"
          @click.stop="emit('delete-file', file)"
        >
          <Trash2 class="size-3" />
        </button>
      </div>
    </div>
  </div>
</template>
