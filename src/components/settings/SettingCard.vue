<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <div
    class="flex w-full cursor-default items-center justify-between gap-3 rounded-xl border border-transparent bg-muted/40 px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] hover:border-border hover:bg-muted/60 hover:shadow-md hover:shadow-black/[0.06] dark:hover:shadow-black/[0.2]"
  >
    <!-- Left: feature description -->
    <div class="min-w-0 flex-1">
      <p class="text-[clamp(11px,1.25vw,12px)] font-medium leading-snug">
        {{ title }}
      </p>
      <p
        v-if="description"
        class="text-muted-foreground mt-[2px] text-[clamp(10px,1.1vw,11px)] leading-relaxed"
      >
        {{ description }}
      </p>
    </div>
    <!-- Right: functional UI control -->
    <div class="shrink-0">
      <slot />
    </div>
  </div>
</template>
