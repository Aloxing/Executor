<script setup lang="ts">
import { CircleAlert, CircleCheck, Info, X } from "lucide-vue-next"
import { computed } from "vue"
import { dismissToast, toasts, type Toast } from "@/lib/toast"

const icons = {
  error: CircleAlert,
  success: CircleCheck,
  info: Info,
}

const iconClasses: Record<Toast["type"], string> = {
  error: "text-destructive",
  success: "text-primary",
  info: "text-muted-foreground",
}

// Newest toast sits closest to the bottom-right corner.
const reversed = computed(() => [...toasts].reverse())
</script>

<template>
  <div
    aria-live="polite"
    class="pointer-events-none fixed right-4 bottom-4 z-[60] flex max-h-[60vh] w-[min(320px,80vw)] flex-col gap-2 overflow-hidden"
  >
    <div
      v-for="toast in reversed"
      :key="toast.id"
      role="alert"
      class="animate-modal-enter bg-popover text-popover-foreground pointer-events-auto flex items-start gap-2 rounded-xl border border-border px-3 py-2.5 shadow-lg shadow-black/[0.08] dark:shadow-black/[0.3]"
    >
      <component
        :is="icons[toast.type]"
        :class="['mt-[1px] size-3.5 shrink-0', iconClasses[toast.type]]"
      />
      <p
        class="min-w-0 flex-1 break-words text-[clamp(11px,1.2vw,12px)] leading-relaxed"
      >
        {{ toast.message }}
      </p>
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground -mt-0.5 -mr-1 inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors duration-200 focus-visible:outline-none"
        aria-label="关闭消息"
        @click="dismissToast(toast.id)"
      >
        <X class="size-3" />
      </button>
    </div>
  </div>
</template>
