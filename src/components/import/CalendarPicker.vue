<script setup lang="ts">
import { ChevronLeft, ChevronRight } from "lucide-vue-next"
import { computed, ref } from "vue"
import { useShortcut } from "@/lib/shortcuts"

const props = defineProps<{
  x: number
  y: number
  /** Selected date as YYYY-MM-DD, empty when nothing is picked. */
  modelValue: string
}>()

const emit = defineEmits<{
  close: []
  /** Empty string clears the selection. */
  pick: [date: string]
}>()

const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"]

function pad(value: number): string {
  return String(value).padStart(2, "0")
}

function format(year: number, monthIndex: number, day: number): string {
  return `${year}-${pad(monthIndex + 1)}-${pad(day)}`
}

function parse(value: string): Date {
  const [year, month, day] = value.split("-").map(Number)
  return new Date(year, month - 1, day)
}

const today = new Date()
const todayStr = format(today.getFullYear(), today.getMonth(), today.getDate())

// The calendar opens on the selected month, or the current one.
const initial = props.modelValue ? parse(props.modelValue) : today
const viewYear = ref(initial.getFullYear())
const viewMonth = ref(initial.getMonth())

// 6x7 grid starting from the Monday before the first day of the month.
const cells = computed(() => {
  const first = new Date(viewYear.value, viewMonth.value, 1)
  const offset = (first.getDay() + 6) % 7
  const list: { date: string; day: number; inMonth: boolean }[] = []
  for (let i = 0; i < 42; i++) {
    const current = new Date(
      viewYear.value,
      viewMonth.value,
      1 - offset + i
    )
    list.push({
      date: format(current.getFullYear(), current.getMonth(), current.getDate()),
      day: current.getDate(),
      inMonth: current.getMonth() === viewMonth.value,
    })
  }
  return list
})

function prevMonth() {
  if (viewMonth.value === 0) {
    viewMonth.value = 11
    viewYear.value--
  } else {
    viewMonth.value--
  }
}

function nextMonth() {
  if (viewMonth.value === 11) {
    viewMonth.value = 0
    viewYear.value++
  } else {
    viewMonth.value++
  }
}

// Closing is driven by the central shortcut system (Esc by default).
useShortcut("close", () => emit("close"))

// Keep the popover inside the viewport (approximate size 252x310).
const position = computed(() => ({
  left: `${Math.max(4, Math.min(props.x, window.innerWidth - 260))}px`,
  top: `${Math.max(4, Math.min(props.y, window.innerHeight - 320))}px`,
}))
</script>

<template>
  <div
    class="fixed inset-0 z-50"
    @click="emit('close')"
    @contextmenu.prevent="emit('close')"
  >
    <div
      role="dialog"
      aria-label="选择日期"
      class="bg-popover text-popover-foreground animate-in fade-in zoom-in-95 absolute w-[252px] rounded-xl border border-border p-2 shadow-md duration-150"
      :style="position"
      @click.stop
    >
      <!-- Month navigation -->
      <div class="flex items-center justify-between px-1 pb-2">
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
          aria-label="上个月"
          @click="prevMonth"
        >
          <ChevronLeft class="size-3.5" />
        </button>
        <p class="text-[clamp(11px,1.25vw,12px)] font-semibold">
          {{ viewYear }}年{{ viewMonth + 1 }}月
        </p>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
          aria-label="下个月"
          @click="nextMonth"
        >
          <ChevronRight class="size-3.5" />
        </button>
      </div>
      <!-- Weekday header -->
      <div class="grid grid-cols-7 px-1 pb-1">
        <span
          v-for="weekday in WEEKDAYS"
          :key="weekday"
          class="text-muted-foreground flex h-6 items-center justify-center text-[clamp(9px,1vw,10px)]"
        >
          {{ weekday }}
        </span>
      </div>
      <!-- Day grid -->
      <div class="grid grid-cols-7 gap-y-0.5 px-1">
        <button
          v-for="cell in cells"
          :key="cell.date"
          type="button"
          class="inline-flex h-7 cursor-pointer items-center justify-center rounded-md text-[clamp(10px,1.1vw,11px)] transition-colors focus-visible:outline-none"
          :class="[
            cell.date === modelValue
              ? 'bg-primary text-primary-foreground font-medium'
              : cell.date === todayStr
                ? 'text-primary hover:bg-primary/10 font-medium'
                : cell.inMonth
                  ? 'hover:bg-muted'
                  : 'text-muted-foreground/50 hover:bg-muted',
          ]"
          @click="emit('pick', cell.date)"
        >
          {{ cell.day }}
        </button>
      </div>
      <!-- Footer quick actions -->
      <div class="mt-1 flex items-center justify-between border-t border-border px-1 pt-1.5">
        <button
          type="button"
          class="hover:bg-muted text-primary inline-flex h-6 cursor-pointer items-center rounded-md bg-transparent px-2 text-[clamp(10px,1.1vw,11px)] font-medium transition-colors focus-visible:outline-none"
          @click="emit('pick', todayStr)"
        >
          今天
        </button>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex h-6 cursor-pointer items-center rounded-md bg-transparent px-2 text-[clamp(10px,1.1vw,11px)] transition-colors focus-visible:outline-none"
          @click="emit('pick', '')"
        >
          清除
        </button>
      </div>
    </div>
  </div>
</template>
