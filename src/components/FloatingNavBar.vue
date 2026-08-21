<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue"

export type NavKey = "import" | "config" | "build" | "output" | "records" | "templates"

interface NavItem {
  key: NavKey
  label: string
  glyph: string
}

const items: NavItem[] = [
  { key: "import", label: "导入区", glyph: "\uE8B5" },
  { key: "config", label: "配置区", glyph: "\uE74C" },
  { key: "build", label: "构建区", glyph: "\uE9D9" },
  { key: "output", label: "产出区", glyph: "\uE896" },
  { key: "records", label: "记录", glyph: "\uE8A4" },
  { key: "templates", label: "模板", glyph: "\uE8A5" },
]

const active = ref<NavKey>("import")
const emit = defineEmits<{
  change: [key: NavKey]
  "open-settings": []
}>()

function select(key: NavKey) {
  if (active.value === key) return
  active.value = key
  emit("change", key)
}

const navRef = ref<HTMLElement | null>(null)
const buttonEls: Partial<Record<NavKey, HTMLButtonElement>> = {}
const indicator = ref({ left: 0, width: 0, ready: false })
let resizeObserver: ResizeObserver | null = null

function setButtonRef(key: NavKey) {
  return (el: unknown) => {
    if (el instanceof HTMLButtonElement) buttonEls[key] = el
    else delete buttonEls[key]
  }
}

function updateIndicator() {
  const nav = navRef.value
  const btn = buttonEls[active.value]
  if (!nav || !btn) return
  const navRect = nav.getBoundingClientRect()
  const btnRect = btn.getBoundingClientRect()
  indicator.value = {
    left: btnRect.left - navRect.left,
    width: btnRect.width,
    ready: true,
  }
}

watch(active, () => {
  nextTick(updateIndicator)
})

// The indicator is a slim capsule line centered under the active button,
// half of the button width.
const lineStyle = computed(() => {
  const width = indicator.value.width * 0.5
  const left = indicator.value.left + (indicator.value.width - width) / 2
  return { left: `${left}px`, width: `${width}px` }
})

onMounted(() => {
  updateIndicator()
  // Icon/label fonts load asynchronously; re-measure once they settle.
  if (document.fonts?.ready) {
    document.fonts.ready.then(() => updateIndicator()).catch(() => {})
  }
  if (typeof ResizeObserver !== "undefined" && navRef.value) {
    resizeObserver = new ResizeObserver(() => updateIndicator())
    resizeObserver.observe(navRef.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
})

defineExpose({ active })
</script>

<template>
  <nav
    ref="navRef"
    class="relative flex items-center gap-1"
    aria-label="主导航"
  >
    <span
      v-if="indicator.ready"
      aria-hidden="true"
      class="bg-primary absolute bottom-0 h-[3px] rounded-full transition-all duration-300 [transition-timing-function:cubic-bezier(0.4,0,0.2,1)]"
      :style="lineStyle"
    />
    <button
      v-for="item in items"
      :key="item.key"
      :ref="setButtonRef(item.key)"
      type="button"
      class="relative z-[1] inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md border-none px-2.5 text-xs font-medium transition-all duration-200 focus-visible:outline-none"
      :class="
        active === item.key
          ? 'text-foreground bg-transparent'
          : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground bg-transparent'
      "
      :aria-label="item.label"
      :aria-current="active === item.key ? 'page' : undefined"
      @click="select(item.key)"
    >
      <span aria-hidden="true" class="win-caption-glyph text-[13px]">
        {{ item.glyph }}
      </span>
      <span>{{ item.label }}</span>
    </button>
    <span aria-hidden="true" class="bg-border mx-1 h-4 w-px shrink-0" />
    <button
      type="button"
      class="hover:bg-accent hover:text-accent-foreground text-muted-foreground relative z-[1] inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
      aria-label="设置"
      title="设置"
      @click="emit('open-settings')"
    >
      <span aria-hidden="true" class="win-caption-glyph text-[13px]">&#xE713;</span>
    </button>
  </nav>
</template>
