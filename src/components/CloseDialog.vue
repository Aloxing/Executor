<script setup lang="ts">
import { X } from "lucide-vue-next"
import { ref } from "vue"
import { useShortcut } from "@/lib/shortcuts"

export type CloseAction = "tray" | "exit"

const emit = defineEmits<{
  select: [action: CloseAction, remember: boolean]
  cancel: []
}>()

const remember = ref(false)

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
      aria-labelledby="close-dialog-title"
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
      <h2 id="close-dialog-title" class="text-[clamp(13px,1.5vw,15px)] font-semibold">
        关闭应用
      </h2>
      <p class="text-muted-foreground mt-2 text-[clamp(11px,1.2vw,13px)] leading-relaxed">
        请选择本次的关闭方式：「隐藏到托盘」后应用继续在后台运行，可从托盘恢复；「彻底关闭」将直接结束应用进程。
      </p>
      <button
        type="button"
        role="checkbox"
        :aria-checked="remember"
        class="text-muted-foreground mt-3 flex cursor-pointer select-none items-center gap-2 border-none bg-transparent p-0 text-left text-[clamp(11px,1.2vw,13px)] focus-visible:outline-none"
        @click="remember = !remember"
      >
        <span
          aria-hidden="true"
          class="flex size-[17px] shrink-0 items-center justify-center rounded-[5px] border transition-all duration-200 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
          :class="
            remember
              ? 'border-primary bg-primary shadow-sm'
              : 'border-input bg-transparent'
          "
        >
          <svg
            v-if="remember"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-primary-foreground size-[11px]"
          >
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </span>
        <span>以后不再询问，并记住我的选择</span>
      </button>
      <div class="mt-4 flex items-center justify-end gap-2">
        <button
          type="button"
          class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('select', 'tray', remember)"
        >
          隐藏到托盘
        </button>
        <button
          type="button"
          class="text-destructive hover:bg-destructive/10 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-destructive/5 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
          @click="emit('select', 'exit', remember)"
        >
          彻底关闭
        </button>
      </div>
    </div>
  </div>
</template>
