<script setup lang="ts">
import { ChevronDown, Plus, Trash2 } from "lucide-vue-next"
import { ref } from "vue"
import SettingSection from "./SettingSection.vue"
import { saveSettings, settings } from "@/lib/settings"
import { showToast } from "@/lib/toast"

// The environment list is collapsible so several versions stay tidy.
const expanded = ref(true)
const adding = ref(false)

/** Last path segment, used as the default environment name. */
function dirName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "")
  return trimmed.split(/[\\/]/).pop() ?? trimmed
}

async function addEnv() {
  if (adding.value) return
  adding.value = true
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择 Gradle 安装目录（bin 内含 gradle 可执行文件）",
    })
    if (!selected) return
    const path = selected as string
    if (settings.gradleEnvs.some((env) => env.path === path)) {
      showToast("该 Gradle 环境已添加")
      return
    }
    settings.gradleEnvs.push({ name: dirName(path), path })
    expanded.value = true
    // Persisted with the settings in the data directory.
    await saveSettings()
    showToast("Gradle 环境已添加", "success")
  } catch (e) {
    showToast(typeof e === "string" ? e : "添加 Gradle 环境失败")
  } finally {
    adding.value = false
  }
}

async function removeEnv(index: number) {
  settings.gradleEnvs.splice(index, 1)
  await saveSettings()
  showToast("Gradle 环境已移除", "success")
}
</script>

<template>
  <div class="space-y-6">
    <SettingSection title="编译">
      <div
        class="flex w-full flex-col rounded-xl border border-transparent bg-muted/40 px-3 py-2.5 transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] hover:border-border hover:bg-muted/60"
      >
        <!-- Card header: title + add + collapse toggle -->
        <div class="flex items-center gap-3">
          <div class="min-w-0 flex-1">
            <p class="text-[clamp(11px,1.25vw,12px)] font-medium leading-snug">
              Gradle 环境
            </p>
            <p
              class="text-muted-foreground mt-[2px] text-[clamp(10px,1.1vw,11px)] leading-relaxed"
            >
              构建区使用的 Gradle 安装目录（gradle 可执行文件位于 bin 中），支持配置多个版本，随设置持久化保存在数据目录
            </p>
          </div>
          <span class="text-muted-foreground shrink-0 text-[clamp(10px,1.1vw,11px)]">
            共 {{ settings.gradleEnvs.length }} 个
          </span>
          <button
            type="button"
            class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex h-7 shrink-0 cursor-pointer items-center gap-1 rounded-md border border-border bg-transparent px-2 text-[clamp(10px,1.1vw,11px)] transition-colors focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="adding"
            @click="addEnv"
          >
            <Plus class="size-3" />
            添加环境
          </button>
          <button
            type="button"
            class="text-muted-foreground hover:bg-accent hover:text-accent-foreground inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md border-none bg-transparent transition-colors focus-visible:outline-none"
            :aria-label="expanded ? '收起环境列表' : '展开环境列表'"
            :title="expanded ? '收起' : '展开'"
            @click="expanded = !expanded"
          >
            <ChevronDown
              class="size-3.5 transition-transform duration-200"
              :class="expanded ? '' : '-rotate-90'"
            />
          </button>
        </div>
        <!-- Environment list -->
        <div v-if="expanded" class="mt-2 flex flex-col gap-1.5">
          <div
            v-for="(env, index) in settings.gradleEnvs"
            :key="env.path"
            class="bg-background/60 flex items-center gap-2 rounded-lg border border-border/60 px-2.5 py-2"
          >
            <p
              class="min-w-0 shrink-0 text-[clamp(10px,1.1vw,11px)] font-semibold"
              :title="env.name"
            >
              {{ env.name }}
            </p>
            <p
              class="text-muted-foreground min-w-0 flex-1 truncate font-mono text-[clamp(9px,1vw,10px)]"
              :title="env.path"
            >
              {{ env.path }}
            </p>
            <button
              type="button"
              class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md transition-colors duration-200 focus-visible:outline-none"
              aria-label="移除环境"
              title="移除该 Gradle 环境"
              @click="removeEnv(index)"
            >
              <Trash2 class="size-3" />
            </button>
          </div>
          <p
            v-if="!settings.gradleEnvs.length"
            class="text-muted-foreground px-1 py-2 text-center text-[clamp(10px,1.1vw,11px)]"
          >
            暂无 Gradle 环境，点击「添加环境」选择安装目录（如 …\gradle-7.5）
          </p>
        </div>
      </div>
    </SettingSection>
  </div>
</template>
