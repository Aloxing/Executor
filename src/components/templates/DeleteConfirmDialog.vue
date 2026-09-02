<script setup lang="ts">
import { X } from "lucide-vue-next"
import { useShortcut } from "@/lib/shortcuts"

defineProps<{
  names: string[]
}>()

const emit = defineEmits<{
  cancel: []
  confirm: []
}>()

// Cancelling is driven by the central shortcut system (Esc by default).
useShortcut("close", () => emit("cancel"))
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0 backdrop-blur-sm"
      aria-hidden="true"
      @click="emit('cancel')"
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-template-title"
      class="animate-modal-enter bg-card text-card-foreground relative w-full max-w-[min(384px,88%)] rounded-2xl border border-border p-[clamp(16px,2.4vh,24px)] shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <button
        type="button"
        class="hover:bg-muted text-muted-foreground hover:text-foreground absolute right-[clamp(10px,1.5vh,14px)] top-[clamp(10px,1.5vh,14px)] inline-flex size-7 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
        aria-label="关闭对话框"
        @click="emit('cancel')"
      >
        <X class="size-3.5" />
      </button>
      <h2 id="delete-template-title" class="text-[clamp(13px,1.5vw,15px)] font-semibold">
        删除模板
      </h2>
      <p class="text-muted-foreground mt-2 text-[clamp(11px,1.2vw,13px)] leading-relaxed">
        <template v-if="names.length === 1">
          确定删除模板「{{ names[0] }}」吗？删除后不可恢复。
        </template>
        <template v-else>
          确定删除所选 {{ names.length }} 个模板吗？删除后不可恢复。
        </template>
      </p>
      <div class="mt-4 flex items-center justify-end gap-2">
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-destructive/5 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('confirm')"
        >
          删除
        </button>
      </div>
    </div>
  </div>
</template>
