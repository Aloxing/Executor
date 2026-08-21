<script setup lang="ts">
import { X } from "lucide-vue-next"
import { computed, onMounted, onUnmounted, ref } from "vue"
import AppSelect from "../AppSelect.vue"
import {
  createTemplate,
  formatNow,
  updateTemplate,
  type TemplateInfo,
} from "@/lib/templates"

const props = defineProps<{
  /** When provided the modal switches to edit mode for this template. */
  initial?: TemplateInfo
}>()

const emit = defineEmits<{
  close: []
  saved: [info: TemplateInfo]
}>()

const editMode = computed(() => !!props.initial)

const name = ref(props.initial?.name ?? "")
const templateType = ref(props.initial?.templateType ?? "android")
const description = ref(props.initial?.description ?? "")
const createdAt = props.initial?.createdAt ?? formatNow()
const updatedAt = formatNow()
const error = ref("")
const saving = ref(false)

const typeOptions = [{ value: "android", label: "Android" }]

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown)
})

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})

async function submit() {
  if (saving.value) return
  if (!name.value.trim()) {
    error.value = "请输入模板名称"
    return
  }
  error.value = ""
  saving.value = true
  try {
    const data = {
      name: name.value,
      templateType: templateType.value,
      description: description.value,
    }
    const info = editMode.value
      ? await updateTemplate(props.initial!.name, data)
      : await createTemplate(data)
    emit("saved", info)
    emit("close")
  } catch (e) {
    error.value =
      typeof e === "string" ? e : editMode.value ? "保存失败，请重试" : "创建失败，请重试"
  } finally {
    saving.value = false
  }
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
      aria-labelledby="create-template-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(90%,460px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="create-template-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          {{ editMode ? "修改模板类型" : "创建模板类型" }}
        </h2>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div class="space-y-3 px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]">
        <div class="space-y-1">
          <label
            for="template-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            模板名称
          </label>
          <input
            id="template-name"
            v-model="name"
            type="text"
            placeholder="请输入模板名称"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            模板类型
          </span>
          <AppSelect
            v-model="templateType"
            :options="typeOptions"
            aria-label="模板类型"
          />
        </div>
        <div class="space-y-1">
          <label
            for="template-description"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            介绍内容
          </label>
          <textarea
            id="template-description"
            v-model="description"
            rows="3"
            placeholder="请输入介绍内容"
            class="bg-background focus-visible:ring-ring w-full resize-none rounded-lg border border-input px-3 py-2 text-[clamp(11px,1.25vw,12px)] leading-relaxed transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div
          class="text-muted-foreground flex items-center justify-between text-[clamp(10px,1.1vw,11px)]"
        >
          <span>创建时间：{{ createdAt }}</span>
          <span>修改时间：{{ updatedAt }}</span>
        </div>
        <p
          v-if="error"
          class="text-destructive text-[clamp(10px,1.1vw,11px)]"
          role="alert"
        >
          {{ error }}
        </p>
      </div>
      <footer
        class="flex shrink-0 items-center justify-end gap-2 border-t border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('close')"
        >
          取消
        </button>
        <button
          type="button"
          class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:opacity-50"
          :disabled="saving"
          @click="submit"
        >
          {{ editMode ? "保存" : "创建" }}
        </button>
      </footer>
    </div>
  </div>
</template>
