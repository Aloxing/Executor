<script setup lang="ts">
import { X } from "lucide-vue-next"
import { onMounted, onUnmounted } from "vue"
import type { AndroidProject } from "@/lib/android"

defineProps<{
  project: AndroidProject
  /** `detach` removes the card from its queue and keeps the project;
   * `delete` (default) destroys the record and the package folder. */
  mode?: "delete" | "detach"
}>()

const emit = defineEmits<{
  cancel: []
  confirm: []
}>()

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("cancel")
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown)
})

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})
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
      aria-labelledby="delete-project-title"
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
      <h2 id="delete-project-title" class="text-[clamp(13px,1.5vw,15px)] font-semibold">
        {{ mode === "detach" ? "从队列删除卡片" : "删除 Android 项目" }}
      </h2>
      <p class="text-muted-foreground mt-2 text-[clamp(11px,1.2vw,13px)] leading-relaxed">
        <template v-if="mode === 'detach'">
          确定将应用「{{ project.appName }}」（{{ project.packageName }}）从队列中删除吗？
          项目信息与包名文件夹会保留在项目目录中。
        </template>
        <template v-else>
          确定删除应用「{{ project.appName }}」（{{ project.packageName }}）吗？
          对应的包名文件夹也会一并删除，删除后不可恢复。
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
          {{ mode === "detach" ? "移除" : "删除" }}
        </button>
      </div>
    </div>
  </div>
</template>
