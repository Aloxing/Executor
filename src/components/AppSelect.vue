<script setup lang="ts">
import { Check, ChevronDown } from "lucide-vue-next"
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue"

const props = defineProps<{
  modelValue: string
  options: { value: string; label: string }[]
  ariaLabel?: string
}>()

const emit = defineEmits<{ "update:modelValue": [value: string] }>()

const open = ref(false)
const rootRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
// Fixed viewport coordinates of the teleported panel.
const panelPos = ref({ left: 0, top: 0, width: 0 })

const current = computed(
  () => props.options.find((o) => o.value === props.modelValue) ?? props.options[0],
)

function openPanel() {
  const root = rootRef.value
  if (!root) return
  const rect = root.getBoundingClientRect()
  const width = Math.max(rect.width, 140)
  // Keep the panel inside the viewport; flip above the trigger when there
  // is not enough room below.
  const estimated = Math.min(props.options.length * 28 + 10, 288)
  let top = rect.bottom + 4
  if (top + estimated > window.innerHeight && rect.top - 4 - estimated >= 0) {
    top = rect.top - 4 - estimated
  }
  panelPos.value = {
    left: Math.max(4, Math.min(rect.left, window.innerWidth - width - 4)),
    top,
    width,
  }
  open.value = true
  // The panel width follows its content; re-clamp once it is measured.
  nextTick(() => {
    const panel = panelRef.value
    if (!panel) return
    const panelRect = panel.getBoundingClientRect()
    if (panelRect.right > window.innerWidth - 4) {
      panelPos.value.left = Math.max(4, window.innerWidth - panelRect.width - 4)
    }
  })
}

// The panel grows with its content so every option fits on one line;
// the trigger width acts as the minimum.
const panelStyle = computed(() => ({
  left: `${panelPos.value.left}px`,
  top: `${panelPos.value.top}px`,
  minWidth: `${panelPos.value.width}px`,
}))

function toggle() {
  open.value ? (open.value = false) : openPanel()
}

function select(value: string) {
  emit("update:modelValue", value)
  open.value = false
}

function onDocumentMousedown(event: MouseEvent) {
  const target = event.target as Node
  if (rootRef.value?.contains(target)) return
  if (panelRef.value?.contains(target)) return
  open.value = false
}

function onDocumentKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") open.value = false
}

watch(open, (visible) => {
  if (visible) {
    document.addEventListener("mousedown", onDocumentMousedown)
    document.addEventListener("keydown", onDocumentKeydown)
  } else {
    document.removeEventListener("mousedown", onDocumentMousedown)
    document.removeEventListener("keydown", onDocumentKeydown)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocumentMousedown)
  document.removeEventListener("keydown", onDocumentKeydown)
})
</script>

<template>
  <div ref="rootRef" class="relative">
    <button
      type="button"
      class="bg-background focus-visible:ring-ring hover:bg-accent/40 flex min-h-[clamp(24px,3.2vh,28px)] w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-input px-[clamp(8px,1vw,10px)] text-left text-[clamp(11px,1.25vw,12px)] leading-none transition-colors focus-visible:outline-none focus-visible:ring-2"
      :aria-label="ariaLabel"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="toggle"
    >
      <!-- Full text on a single line, never wrapping or truncating;
           leading-none keeps the glyph vertically centered in the box. -->
      <span class="min-w-0 leading-none whitespace-nowrap">{{ current?.label }}</span>
      <ChevronDown
        class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-200"
        :class="open ? 'rotate-180' : ''"
      />
    </button>
    <!-- Teleported above every layer (modals are z-50) so the list is
         never clipped by scroll containers or hidden behind cards. -->
    <Teleport to="body">
      <div
        v-if="open"
        ref="panelRef"
        role="listbox"
        class="bg-popover text-popover-foreground animate-in fade-in zoom-in-95 fixed z-[70] max-h-[240px] overflow-y-auto rounded-md border border-border p-1 shadow-md duration-150"
        :style="{ ...panelStyle, width: 'max-content', whiteSpace: 'nowrap' }"
      >
        <button
          v-for="option in options"
          :key="option.value"
          type="button"
          role="option"
          class="flex min-h-[clamp(24px,3.2vh,28px)] w-full cursor-pointer items-center justify-between gap-2 rounded-[4px] px-2 text-left text-[clamp(11px,1.25vw,12px)] leading-none transition-colors"
          :class="
            option.value === modelValue
              ? 'bg-accent text-accent-foreground font-medium'
              : 'hover:bg-accent/60 text-foreground'
          "
          :aria-selected="option.value === modelValue"
          @click="select(option.value)"
        >
          <!-- Full text on a single line; the panel width follows it -->
          <span class="leading-none whitespace-nowrap">{{ option.label }}</span>
          <Check
            v-if="option.value === modelValue"
            class="size-3.5 shrink-0"
          />
        </button>
      </div>
    </Teleport>
  </div>
</template>
