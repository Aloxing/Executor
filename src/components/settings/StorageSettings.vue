<script setup lang="ts">
import { onMounted, ref, watch } from "vue"
import { FolderOpen } from "lucide-vue-next"
import SettingSection from "./SettingSection.vue"
import SettingCard from "./SettingCard.vue"
import { getDataDir, saveSettings, setDataDir, settings } from "@/lib/settings"

const dataDir = ref("")
const changing = ref(false)
const choosingWorkspace = ref(false)

onMounted(async () => {
  dataDir.value = await getDataDir()
})

watch(
  () => settings.workspacePath,
  () => {
    saveSettings()
  }
)

async function browseDataDir() {
  changing.value = true
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择数据存储目录",
      defaultPath: dataDir.value || undefined,
    })
    if (selected) {
      const newPath = await setDataDir(selected as string)
      dataDir.value = newPath
    }
  } catch {
    // Not running inside Tauri or dialog cancelled.
  } finally {
    changing.value = false
  }
}

async function browseWorkspace() {
  choosingWorkspace.value = true
  try {
    const { open } = await import("@tauri-apps/plugin-dialog")
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择工作空间目录",
      defaultPath: settings.workspacePath || undefined,
    })
    if (selected) {
      settings.workspacePath = selected as string
    }
  } catch {
    // Not running inside Tauri or dialog cancelled.
  } finally {
    choosingWorkspace.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <SettingSection title="数据存储">
      <SettingCard
        title="数据目录"
        description="设置、任务记录等数据的存储路径，更改后现有数据会自动迁移"
      >
        <div class="flex items-center gap-2">
          <span
            class="text-muted-foreground max-w-[180px] truncate text-[clamp(10px,1.1vw,11px)]"
            :title="dataDir"
          >
            {{ dataDir || "加载中…" }}
          </span>
          <button
            type="button"
            class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex size-6 cursor-pointer items-center justify-center rounded-md border border-border bg-transparent transition-colors focus-visible:outline-none"
            aria-label="浏览目录"
            :disabled="changing"
            @click="browseDataDir"
          >
            <FolderOpen class="size-3" />
          </button>
        </div>
      </SettingCard>
    </SettingSection>
    <SettingSection title="工作空间">
      <SettingCard
        title="工作空间路径"
        description="项目文件的默认工作目录，导入与构建操作将基于此路径"
      >
        <div class="flex items-center gap-2">
          <span
            class="text-muted-foreground max-w-[180px] truncate text-[clamp(10px,1.1vw,11px)]"
            :title="settings.workspacePath"
          >
            {{ settings.workspacePath || "未选择" }}
          </span>
          <button
            type="button"
            class="hover:bg-accent hover:text-accent-foreground text-muted-foreground inline-flex size-6 cursor-pointer items-center justify-center rounded-md border border-border bg-transparent transition-colors focus-visible:outline-none"
            aria-label="选择工作空间"
            :disabled="choosingWorkspace"
            @click="browseWorkspace"
          >
            <FolderOpen class="size-3" />
          </button>
        </div>
      </SettingCard>
    </SettingSection>
  </div>
</template>
