<script setup lang="ts">
import { Smartphone, X } from "lucide-vue-next"
import { useShortcut } from "@/lib/shortcuts"
import type { AndroidDevice } from "@/lib/devices"

// Chooser shown when several adb devices are connected: pick the phone
// that receives the apk (adb install -s <serial>).
defineProps<{
  fileName: string
  devices: AndroidDevice[]
}>()

const emit = defineEmits<{
  close: []
  pick: [serial: string]
}>()

// Closing is driven by the central shortcut system (Esc by default).
useShortcut("close", () => emit("close"))
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0 backdrop-blur-sm"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="install-target-title"
      class="animate-modal-enter bg-card text-card-foreground relative w-full max-w-[min(420px,90%)] rounded-2xl border border-border p-[clamp(16px,2.4vh,24px)] shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <button
        type="button"
        class="hover:bg-muted text-muted-foreground hover:text-foreground absolute right-[clamp(10px,1.5vh,14px)] top-[clamp(10px,1.5vh,14px)] inline-flex size-7 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
        aria-label="关闭对话框"
        @click="emit('close')"
      >
        <X class="size-3.5" />
      </button>
      <h2
        id="install-target-title"
        class="text-[clamp(13px,1.5vw,15px)] font-semibold"
      >
        选择安装设备
      </h2>
      <p
        class="text-muted-foreground mt-1.5 truncate text-[clamp(11px,1.2vw,12px)]"
        :title="fileName"
      >
        检测到多台设备，将「{{ fileName }}」安装到：
      </p>
      <div class="mt-3 flex flex-col gap-1.5">
        <button
          v-for="device in devices"
          :key="device.serial"
          type="button"
          class="bg-background/60 hover:bg-muted/60 hover:border-border flex w-full cursor-pointer items-center gap-2.5 rounded-lg border border-border/60 px-2.5 py-2 text-left transition-colors duration-200 focus-visible:outline-none"
          @click="emit('pick', device.serial)"
        >
          <Smartphone class="text-primary size-4 shrink-0" />
          <span class="min-w-0 flex-1">
            <span
              class="block truncate text-[clamp(11px,1.25vw,12px)] font-medium"
            >
              {{ device.model || device.product || "未知设备" }}
            </span>
            <span
              class="text-muted-foreground block truncate font-mono text-[clamp(9px,1vw,10px)]"
            >
              {{ device.serial }}
            </span>
          </span>
        </button>
      </div>
    </div>
  </div>
</template>
