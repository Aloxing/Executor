<script setup lang="ts">
import { RefreshCw, X } from "lucide-vue-next"
import { ref } from "vue"
import AppSelect from "../AppSelect.vue"
import { useShortcut } from "@/lib/shortcuts"
import {
  createImportQueue,
  generateUuid,
  type ImportQueue,
} from "@/lib/queues"
import { formatNow } from "@/lib/templates"

const emit = defineEmits<{
  close: []
  saved: [queue: ImportQueue]
}>()

const name = ref("")
const uuid = ref(generateUuid())
const queueType = ref("android")
// Creation time is fixed when the modal opens.
const createdAt = formatNow()
const error = ref("")
const saving = ref(false)

const typeOptions = [{ value: "android", label: "Android" }]

// Closing and saving are driven by the central shortcut system.
useShortcut("close", () => emit("close"))
useShortcut("save", submit)

async function submit() {
  if (saving.value) return
  if (!name.value.trim()) {
    error.value = "请输入队列名称"
    return
  }
  error.value = ""
  saving.value = true
  try {
    const queue = await createImportQueue({
      name: name.value,
      uuid: uuid.value,
      queueType: queueType.value,
    })
    emit("saved", queue)
    emit("close")
  } catch (e) {
    error.value = typeof e === "string" ? e : "创建失败，请重试"
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
      aria-labelledby="create-queue-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex w-[min(90%,460px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="create-queue-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          创建导入队列
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
            for="queue-name"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            队列名称
          </label>
          <input
            id="queue-name"
            v-model="name"
            type="text"
            placeholder="请输入队列名称"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-3 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
        </div>
        <div class="space-y-1">
          <label
            for="queue-uuid"
            class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]"
          >
            队列编号
          </label>
          <div class="flex items-center gap-2">
            <input
              id="queue-uuid"
              v-model="uuid"
              type="text"
              readonly
              aria-label="队列编号"
              class="bg-muted/40 text-muted-foreground h-8 w-full rounded-lg border border-input px-3 font-mono text-[clamp(10px,1.1vw,11px)] focus-visible:outline-none"
            />
            <button
              type="button"
              class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-muted/60 transition-colors duration-200 focus-visible:outline-none"
              aria-label="重新生成队列编号"
              title="重新生成"
              @click="uuid = generateUuid()"
            >
              <RefreshCw class="size-3.5" />
            </button>
          </div>
        </div>
        <div class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            队列类型
          </span>
          <AppSelect
            v-model="queueType"
            :options="typeOptions"
            aria-label="队列类型"
          />
        </div>
        <div class="space-y-1">
          <span class="text-muted-foreground block text-[clamp(10px,1.1vw,11px)]">
            创建时间
          </span>
          <p class="bg-muted/40 text-muted-foreground h-8 rounded-lg border border-input px-3 leading-8 text-[clamp(11px,1.25vw,12px)]">
            {{ createdAt }}
          </p>
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
          创建
        </button>
      </footer>
    </div>
  </div>
</template>
