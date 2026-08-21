<script setup lang="ts">
import { watch } from "vue"
import SettingSection from "./SettingSection.vue"
import SettingCard from "./SettingCard.vue"
import SettingSelect from "./SettingSelect.vue"
import { saveSettings, settings, type CloseBehavior } from "@/lib/settings"

const closeOptions: { value: CloseBehavior; label: string }[] = [
  { value: "ask", label: "每次询问" },
  { value: "tray", label: "隐藏到托盘" },
  { value: "exit", label: "彻底退出" },
]

watch(
  () => settings.closeBehavior,
  () => {
    saveSettings()
  }
)
</script>

<template>
  <div class="space-y-6">
    <SettingSection title="关闭行为">
      <SettingCard
        title="窗口关闭方式"
        description="选择关闭主窗口时应用的处理方式，「每次询问」会弹窗让你选择"
      >
        <SettingSelect
          v-model="settings.closeBehavior"
          :options="closeOptions"
        />
      </SettingCard>
    </SettingSection>
  </div>
</template>
