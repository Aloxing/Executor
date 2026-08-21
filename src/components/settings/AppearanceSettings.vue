<script setup lang="ts">
import { watch } from "vue"
import SettingSection from "./SettingSection.vue"
import SettingCard from "./SettingCard.vue"
import SettingSelect from "./SettingSelect.vue"
import { saveSettings, settings, type ThemeMode } from "@/lib/settings"
import { applyThemeMode } from "@/lib/theme"

const themeOptions: { value: ThemeMode; label: string }[] = [
  { value: "dark", label: "深色" },
  { value: "light", label: "浅色" },
  { value: "system", label: "跟随系统" },
]

watch(
  () => settings.themeMode,
  (mode) => {
    applyThemeMode(mode)
    saveSettings()
  }
)
</script>

<template>
  <div class="space-y-6">
    <SettingSection title="应用主题">
      <SettingCard
        title="主题模式"
        description="选择应用的颜色外观，「跟随系统」会随系统深浅色设置自动切换"
      >
        <SettingSelect v-model="settings.themeMode" :options="themeOptions" />
      </SettingCard>
    </SettingSection>
  </div>
</template>
