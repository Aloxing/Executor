<script setup lang="ts">
import { Check, X } from "lucide-vue-next"
import { onMounted, onUnmounted, ref } from "vue"
import { listTemplates, type TemplateInfo } from "@/lib/templates"
import { formatNow } from "@/lib/templates"

defineProps<{
  /** Name of the sub project being configured; shown in the title. */
  projectName: string
}>()

const emit = defineEmits<{
  close: []
  /** `start` decides between 保存并开始配置 and 仅保存. */
  save: [templateName: string, start: boolean]
}>()

const templates = ref<TemplateInfo[]>([])
const selectedName = ref("")
const saving = ref(false)
// Modify-config time defaults to the moment the modal opens.
const configTime = formatNow()

onMounted(async () => {
  window.addEventListener("keydown", onKeydown)
  templates.value = await listTemplates()
})

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})

function submit(start: boolean) {
  if (saving.value || !selectedName.value) return
  saving.value = true
  emit("save", selectedName.value, start)
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-[3%]">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0 backdrop-blur-sm"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="select-template-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex max-h-[min(86%,680px)] w-[min(90%,520px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="select-template-title"
          class="min-w-0 truncate text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          选择配置模板 · {{ projectName }}
        </h2>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <!-- Scrollable directory of template cards from the templates page -->
      <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]">
        <button
          v-for="template in templates"
          :key="template.name"
          type="button"
          class="flex w-full cursor-pointer select-none flex-col gap-1.5 rounded-xl border px-3 py-2.5 text-left transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
          :class="
            selectedName === template.name
              ? 'border-primary/40 bg-primary/[0.06]'
              : 'border-border bg-muted/40 hover:bg-muted/60'
          "
          @click="selectedName = template.name"
        >
          <div class="flex items-center gap-2">
            <span
              aria-hidden="true"
              class="flex size-4 shrink-0 items-center justify-center rounded-[4px] border transition-all duration-200 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
              :class="
                selectedName === template.name
                  ? 'border-primary bg-primary'
                  : 'border-input bg-transparent'
              "
            >
              <Check
                v-if="selectedName === template.name"
                class="text-primary-foreground size-[10px]"
                :stroke-width="3.5"
              />
            </span>
            <p
              class="min-w-0 flex-1 truncate text-[clamp(11px,1.2vw,12px)] font-semibold"
              :title="template.name"
            >
              {{ template.name }}
            </p>
            <span
              v-if="template.templateType"
              class="bg-muted text-muted-foreground shrink-0 rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium"
            >
              {{ template.templateType }}
            </span>
          </div>
          <p
            v-if="template.description"
            class="text-muted-foreground truncate pl-6 text-[clamp(9px,1vw,10px)]"
            :title="template.description"
          >
            {{ template.description }}
          </p>
        </button>
        <p
          v-if="!templates.length"
          class="text-muted-foreground flex-1 py-6 text-center text-[clamp(10px,1.1vw,11px)]"
        >
          暂无模板，请先在模板页面创建模板
        </p>
      </div>
      <footer
        class="flex shrink-0 items-center justify-between gap-2 border-t border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <!-- Modify-config time recorded on save; defaults to now. -->
        <p
          class="text-muted-foreground min-w-0 truncate text-[clamp(10px,1.1vw,11px)]"
          :title="`修改配置时间：${configTime}`"
        >
          修改配置时间：{{ configTime }}
        </p>
        <div class="flex shrink-0 items-center gap-2">
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="emit('close')"
          >
            取消
          </button>
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="saving || !selectedName"
            @click="submit(false)"
          >
            仅保存
          </button>
          <button
            type="button"
            class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 cursor-pointer items-center justify-center rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="saving || !selectedName"
            @click="submit(true)"
          >
            保存并开始配置
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>
