<script setup lang="ts" generic="T extends string">
defineProps<{
  modelValue: T
  options: { value: T; label: string }[]
}>()

const emit = defineEmits<{
  "update:modelValue": [value: T]
}>()
</script>

<template>
  <div
    class="bg-muted inline-flex items-center gap-0.5 rounded-md p-0.5"
    role="radiogroup"
  >
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      role="radio"
      :aria-checked="modelValue === option.value"
      class="inline-flex h-[clamp(22px,3.5vh,26px)] cursor-pointer items-center rounded-[5px] border-none px-[clamp(8px,1vw,12px)] text-[clamp(10px,1.1vw,11px)] font-medium transition-all duration-150 focus-visible:outline-none"
      :class="
        modelValue === option.value
          ? 'bg-card text-card-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground bg-transparent'
      "
      @click="emit('update:modelValue', option.value)"
    >
      {{ option.label }}
    </button>
  </div>
</template>
