<script setup lang="ts">
import { Hammer, X } from "lucide-vue-next"
import { computed, ref } from "vue"
import AppSelect from "../AppSelect.vue"
import { buildCommands, type BuildQueue } from "@/lib/build"
import { useShortcut } from "@/lib/shortcuts"

const props = defineProps<{
  queue: BuildQueue
}>()

const emit = defineEmits<{
  close: []
  /** `mode`: serial = one project after another, parallel = all at once. */
  start: [args: string[], mode: "serial" | "parallel"]
}>()

// Gradle task preset; every project runs wrapper → gradlew <args>.
const commandLabel = ref(buildCommands[0].label)
const commandOptions = buildCommands.map((c) => ({
  value: c.label,
  label: c.label,
}))

// Compile mode: serial is the stable default, parallel is faster but
// runs one gradle process per project at the same time.
const mode = ref<"serial" | "parallel">("serial")

const projectCount = computed(() => props.queue.projects.length)

function start() {
  const command = buildCommands.find((c) => c.label === commandLabel.value)
  if (!command || !projectCount.value) return
  emit("start", command.args, mode.value)
}

// Closing and starting are driven by the central shortcut system.
useShortcut("close", () => emit("close"))
useShortcut("save", start)
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-[3%]">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(480px,92%)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-4 py-3"
      >
        <h2 class="text-[clamp(12px,1.5vw,13px)] font-semibold">
          全部构建 · {{ queue.name }}
        </h2>
        <button
          type="button"
          class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div class="flex flex-col gap-3 p-4">
        <!-- Gradle task preset -->
        <div class="flex items-center gap-2">
          <span
            class="text-muted-foreground w-16 shrink-0 text-[clamp(11px,1.25vw,12px)]"
          >
            构建命令
          </span>
          <AppSelect
            v-model="commandLabel"
            :options="commandOptions"
            aria-label="构建命令"
          />
        </div>
        <p
          class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] leading-relaxed"
        >
          每个项目依次执行 gradle wrapper → gradlew {{ commandLabel }}，共
          {{ projectCount }} 个项目
        </p>
        <!-- Compile mode -->
        <div class="flex flex-col gap-2">
          <button
            type="button"
            class="flex w-full cursor-pointer flex-col gap-0.5 rounded-xl border px-3 py-2.5 text-left transition-colors duration-200 focus-visible:outline-none"
            :class="
              mode === 'serial'
                ? 'border-primary/50 bg-primary/[0.06]'
                : 'border-border bg-transparent hover:bg-muted/50'
            "
            @click="mode = 'serial'"
          >
            <span class="text-[clamp(11px,1.25vw,12px)] font-semibold">
              串行编译（推荐）
            </span>
            <span
              class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] leading-relaxed"
            >
              上一个项目编译完成后再开始下一个；资源占用平稳，可随时停止中断后续
            </span>
          </button>
          <button
            type="button"
            class="flex w-full cursor-pointer flex-col gap-0.5 rounded-xl border px-3 py-2.5 text-left transition-colors duration-200 focus-visible:outline-none"
            :class="
              mode === 'parallel'
                ? 'border-primary/50 bg-primary/[0.06]'
                : 'border-border bg-transparent hover:bg-muted/50'
            "
            @click="mode = 'parallel'"
          >
            <span class="text-[clamp(11px,1.25vw,12px)] font-semibold">
              并行编译
            </span>
            <span
              class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] leading-relaxed"
            >
              全部项目同时编译，速度最快；CPU/内存/磁盘占用高，项目多时慎用
            </span>
          </button>
        </div>
      </div>
      <footer
        class="flex shrink-0 items-center justify-end gap-2 border-t border-border px-4 py-3"
      >
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('close')"
        >
          取消
        </button>
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="!projectCount"
          @click="start"
        >
          <Hammer class="size-3.5" />
          开始构建{{ projectCount ? `（${projectCount} 个项目）` : "" }}
        </button>
      </footer>
    </div>
  </div>
</template>
