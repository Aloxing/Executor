<script setup lang="ts">
import { Loader2, Search, X } from "lucide-vue-next"
import { computed, onMounted, onUnmounted, ref } from "vue"
import { listAndroidProjects, type AndroidProject } from "@/lib/android"

const emit = defineEmits<{
  close: []
  confirm: [projects: AndroidProject[]]
}>()

const keyword = ref("")
const projects = ref<AndroidProject[]>([])
const selected = ref<Set<string>>(new Set())
const confirming = ref(false)

// Only projects that finished importing can be configured from.
onMounted(async () => {
  window.addEventListener("keydown", onKeydown)
  const all = await listAndroidProjects()
  projects.value = all.filter((p) => p.importStatus === "imported")
})

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close")
}

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown)
})

// Search matches the app name and the package name, like the import page.
const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return projects.value
  return projects.value.filter(
    (p) =>
      p.packageName.toLowerCase().includes(kw) ||
      p.appName.toLowerCase().includes(kw)
  )
})

function toggle(packageName: string) {
  const next = new Set(selected.value)
  next.has(packageName) ? next.delete(packageName) : next.add(packageName)
  selected.value = next
}

function submit() {
  if (confirming.value || !selected.value.size) return
  confirming.value = true
  const picks = projects.value.filter((p) => selected.value.has(p.packageName))
  emit("confirm", picks)
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-[3%]">
    <div
      class="animate-backdrop-fade bg-black/40 absolute inset-0 backdrop-blur-sm"
      aria-hidden="true"
      @click="emit('close')"
    />
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="pick-imported-title"
      class="animate-modal-enter bg-card text-card-foreground relative flex max-h-[min(86%,680px)] w-[min(90%,480px)] flex-col rounded-2xl border border-border shadow-2xl shadow-black/[0.12] dark:shadow-black/[0.4]"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <h2
          id="pick-imported-title"
          class="text-[clamp(12px,1.5vw,13px)] font-semibold"
        >
          从已导入的项目配置
        </h2>
        <button
          type="button"
          class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex size-7 cursor-pointer items-center justify-center rounded-lg border-none bg-transparent transition-colors duration-200 focus-visible:outline-none"
          aria-label="关闭"
          @click="emit('close')"
        >
          <X class="size-3.5" />
        </button>
      </header>
      <div class="flex min-h-0 flex-1 flex-col gap-2 px-[clamp(14px,2vw,18px)] py-[clamp(12px,2vh,16px)]">
        <!-- Search box on top of the card directory -->
        <div class="relative shrink-0">
          <Search
            class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
          />
          <input
            v-model="keyword"
            type="text"
            placeholder="搜索项目，支持名称或包名"
            class="bg-background focus-visible:ring-ring h-8 w-full rounded-lg border border-input pr-8 pl-8 text-[clamp(11px,1.25vw,12px)] transition-colors focus-visible:outline-none focus-visible:ring-2"
          />
          <button
            v-if="keyword"
            type="button"
            class="text-muted-foreground hover:text-foreground absolute top-1/2 right-1.5 inline-flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-transparent transition-colors focus-visible:outline-none"
            aria-label="清空搜索"
            @click="keyword = ''"
          >
            <X class="size-3" />
          </button>
        </div>
        <!-- Scrollable directory of imported project cards -->
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-0.5">
          <button
            v-for="project in filtered"
            :key="project.packageName"
            type="button"
            class="flex w-full cursor-pointer select-none flex-col gap-1.5 rounded-xl border px-3 py-2.5 text-left transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
            :class="
              selected.has(project.packageName)
                ? 'border-primary/40 bg-primary/[0.06]'
                : 'border-border bg-muted/40 hover:bg-muted/60'
            "
            @click="toggle(project.packageName)"
          >
            <div class="flex items-center gap-2">
              <!-- Selection checkbox -->
              <span
                aria-hidden="true"
                class="flex size-4 shrink-0 items-center justify-center rounded-[4px] border transition-all duration-200 ease-[cubic-bezier(0.25,0.1,0.25,1)]"
                :class="
                  selected.has(project.packageName)
                    ? 'border-primary bg-primary'
                    : 'border-input bg-transparent'
                "
              >
                <svg
                  v-if="selected.has(project.packageName)"
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
              <p
                class="min-w-0 flex-1 truncate text-[clamp(11px,1.2vw,12px)] font-semibold"
                :title="project.appName"
              >
                {{ project.appName }}
              </p>
              <span
                class="shrink-0 rounded-md bg-emerald-500/15 px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium text-emerald-600 dark:text-emerald-500"
              >
                已导入
              </span>
            </div>
            <p
              class="text-muted-foreground truncate pl-6 font-mono text-[clamp(9px,1vw,10px)]"
              :title="project.packageName"
            >
              {{ project.packageName }}
            </p>
          </button>
          <p
            v-if="!filtered.length"
            class="text-muted-foreground flex-1 py-6 text-center text-[clamp(10px,1.1vw,11px)]"
          >
            {{
              projects.length
                ? "未找到匹配的项目"
                : "暂无已导入的项目，请先在导入区完成导入"
            }}
          </p>
        </div>
      </div>
      <footer
        class="flex shrink-0 items-center justify-between gap-2 border-t border-border px-[clamp(14px,2vw,18px)] py-[clamp(10px,1.6vh,14px)]"
      >
        <p
          class="text-muted-foreground min-w-0 truncate text-[clamp(10px,1.1vw,11px)]"
        >
          已选 {{ selected.size }} 项
        </p>
        <div class="flex shrink-0 items-center gap-2">
          <button
            type="button"
            class="hover:bg-muted inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center rounded-lg bg-muted/60 px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            @click="emit('close')"
          >
            取消
          </button>
          <button
            type="button"
            class="bg-primary text-primary-foreground hover:bg-primary/90 inline-flex h-8 min-w-[80px] cursor-pointer items-center justify-center gap-1.5 rounded-lg px-3 text-[clamp(11px,1.25vw,13px)] font-medium transition-colors duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="confirming || !selected.size"
            @click="submit"
          >
            <Loader2 v-if="confirming" class="size-3.5 animate-spin" />
            {{ confirming ? "添加中…" : "确定" }}
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>
