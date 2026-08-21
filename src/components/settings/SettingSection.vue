<script setup lang="ts">
defineProps<{
  title: string
}>()
</script>

<template>
  <section class="space-y-[clamp(6px,1vh,10px)]">
    <h3 class="text-[clamp(11px,1.3vw,13px)] font-semibold">{{ title }}</h3>
    <slot />
  </section>
</template>
