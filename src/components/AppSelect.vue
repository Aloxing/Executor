<script setup lang="ts">
import { Check, ChevronDown } from "lucide-vue-next"
import { computed, onBeforeUnmount, ref, watch } from "vue"

const props = defineProps<{
  modelValue: string
  options: { value: string; label: string }[]
  ariaLabel?: string
}>()

const emit = defineEmits<{ "update:modelValue": [value: string] }>()

const open = ref(false)
const rootRef = ref<HTMLElement | null>(null)

const current = computed(
  () => props.options.find((o) => o.value === props.modelValue) ?? props.options[0],
)

function toggle() {
  open.value = !open.value
}

function select(value: string) {
  emit("update:modelValue", value)
  open.value = false
}

function onDocumentMousedown(event: MouseEvent) {
  if (rootRef.value && !rootRef.value.contains(event.target as Node)) {
    open.value = false
  }
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
      class="bg-background focus-visible:ring-ring hover:bg-accent/40 flex h-[clamp(26px,4vh,32px)] w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-input px-[clamp(8px,1vw,10px)] text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
      :aria-label="ariaLabel"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="toggle"
    >
      <span class="truncate">{{ current?.label }}</span>
      <ChevronDown
        class="text-muted-foreground size-3.5 shrink-0 transition-transform duration-200"
        :class="open ? 'rotate-180' : ''"
      />
    </button>
    <div
      v-if="open"
      role="listbox"
      class="bg-popover animate-in fade-in zoom-in-95 absolute top-full right-0 z-20 mt-1 w-full min-w-[clamp(140px,16vw,180px)] rounded-md border border-border p-1 shadow-md duration-150"
    >
      <button
        v-for="option in options"
        :key="option.value"
        type="button"
        role="option"
        class="flex w-full cursor-pointer items-center justify-between gap-2 rounded-[4px] px-2 py-[clamp(4px,0.8vh,6px)] text-left text-[clamp(11px,1.25vw,12px)] transition-colors"
        :class="
          option.value === modelValue
            ? 'bg-accent text-accent-foreground font-medium'
            : 'hover:bg-accent/60 text-foreground'
        "
        :aria-selected="option.value === modelValue"
        @click="select(option.value)"
      >
        <span class="truncate">{{ option.label }}</span>
        <Check
          v-if="option.value === modelValue"
          class="size-3.5 shrink-0"
        />
      </button>
    </div>
  </div>
</template>
