<script setup lang="ts">
import { Smartphone, X } from "lucide-vue-next"
import { computed, ref } from "vue"
import type { AndroidDevice } from "@/lib/devices"
import { useShortcut } from "@/lib/shortcuts"

const props = defineProps<{
  /** Detected and authorized USB-debug devices. */
  devices: AndroidDevice[]
}>()

const emit = defineEmits<{
  close: []
  start: [serials: string[], packageName: string]
}>()

// All detected devices are checked by default.
const selected = ref<Set<string>>(new Set(props.devices.map((d) => d.serial)))
// Optional app filter: empty = whole-device logcat.
const packageName = ref("")

const selectedCount = computed(() => selected.value.size)

function toggle(serial: string) {
  const next = new Set(selected.value)
  if (next.has(serial)) next.delete(serial)
  else next.add(serial)
  selected.value = next
}

function start() {
  if (!selectedCount.value) return
  emit("start", [...selected.value], packageName.value.trim())
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
          抓取设备日志
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
        <!-- Device checklist: one log page per checked device -->
        <div class="flex flex-col gap-1.5">
          <p
            class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] font-medium"
          >
            已检测到的设备（每台设备一个日志页）
          </p>
          <button
            v-for="device in devices"
            :key="device.serial"
            type="button"
            class="flex w-full cursor-pointer items-center gap-2 rounded-xl border px-3 py-2 text-left transition-colors duration-200 focus-visible:outline-none"
            :class="
              selected.has(device.serial)
                ? 'border-primary/50 bg-primary/[0.06]'
                : 'border-border bg-transparent hover:bg-muted/50'
            "
            @click="toggle(device.serial)"
          >
            <span
              aria-hidden="true"
              class="flex size-4 shrink-0 items-center justify-center rounded-[4px] border transition-all duration-200"
              :class="
                selected.has(device.serial)
                  ? 'border-primary bg-primary'
                  : 'border-input bg-transparent'
              "
            >
              <svg
                v-if="selected.has(device.serial)"
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
            <Smartphone class="text-muted-foreground size-3.5 shrink-0" />
            <span
              class="min-w-0 flex-1 truncate text-[clamp(11px,1.25vw,12px)] font-semibold"
            >
              {{ device.model || device.product || "Android 设备" }}
            </span>
            <span
              class="text-muted-foreground shrink-0 font-mono text-[clamp(9px,1vw,10px)]"
            >
              {{ device.serial }}
            </span>
          </button>
        </div>
        <!-- Optional app filter -->
        <div class="flex flex-col gap-1">
          <label
            class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] font-medium"
            for="device-log-package"
          >
            应用包名过滤（可选）
          </label>
          <input
            id="device-log-package"
            v-model="packageName"
            type="text"
            placeholder="如 com.example.app，留空抓取整机日志"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input px-2.5 font-mono text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
          <p
            class="text-muted-foreground text-[clamp(9px,1vw,10px)] leading-relaxed"
          >
            填写包名后只抓取该应用的日志（按进程附加）：应用未启动会自动等待，崩溃或重启后自动重连；留空则抓取整机日志，系统噪音较多
          </p>
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
          :disabled="!selectedCount"
          @click="start"
        >
          <Smartphone class="size-3.5" />
          开始抓取{{ selectedCount ? `（${selectedCount} 台设备）` : "" }}
        </button>
      </footer>
    </div>
  </div>
</template>
