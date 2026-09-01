<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean
  /** Tailwind background class for the on state (defaults to primary). */
  activeClass?: string
}>()

const emit = defineEmits<{
  "update:modelValue": [value: boolean]
}>()

function toggle() {
  emit("update:modelValue", !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    class="relative inline-flex h-[18px] w-[32px] shrink-0 cursor-pointer items-center rounded-full border-none transition-colors duration-200 focus-visible:outline-none"
    :class="modelValue ? activeClass ?? 'bg-primary' : 'bg-muted'"
    @click="toggle"
  >
    <span
      class="inline-block size-[14px] rounded-full bg-white shadow-sm transition-transform duration-200"
      :class="modelValue ? 'translate-x-[15px]' : 'translate-x-[2px]'"
    />
  </button>
</template>
