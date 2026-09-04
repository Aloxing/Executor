<script setup lang="ts">
import { RotateCcw } from "lucide-vue-next"
import { onUnmounted, ref, watch } from "vue"
import SettingSection from "./SettingSection.vue"
import SettingCard from "./SettingCard.vue"
import SettingSelect from "./SettingSelect.vue"
import SettingSwitch from "./SettingSwitch.vue"
import { saveSettings, settings, type CloseBehavior } from "@/lib/settings"
import {
  SHORTCUT_LABELS,
  SHORTCUT_ORDER,
  formatCombo,
  normalizeCombo,
  resetAllShortcuts,
  resetShortcut,
  setShortcut,
  shortcutFor,
} from "@/lib/shortcuts"
import { showToast } from "@/lib/toast"

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

watch(
  () => settings.systemNotify,
  () => {
    saveSettings()
  }
)

// --- Shortcut customization ----------------------------------------------------

// The action currently capturing a new key combo (empty = none).
const listening = ref("")

function onCapture(event: KeyboardEvent) {
  // Capture phase + stopImmediatePropagation keeps the global dispatcher
  // from acting on the very keys being recorded.
  event.preventDefault()
  event.stopImmediatePropagation()
  const action = listening.value
  if (!action) return
  if (event.key === "Escape") {
    stopListening()
    return
  }
  if (event.key === "Backspace" || event.key === "Delete") {
    setShortcut(action, "")
    stopListening()
    showToast("已清除该快捷键")
    return
  }
  const combo = normalizeCombo(event)
  // Modifier-only presses keep the capture waiting for the full combo.
  if (!combo) return
  const error = setShortcut(action, combo)
  stopListening()
  if (error) showToast(error)
  else showToast(`已绑定 ${formatCombo(combo)}`, "success")
}

function startListening(action: string) {
  if (listening.value) stopListening()
  listening.value = action
  window.addEventListener("keydown", onCapture, { capture: true })
}

function stopListening() {
  listening.value = ""
  window.removeEventListener("keydown", onCapture, { capture: true })
}

function isCustomized(action: string): boolean {
  return settings.shortcuts[action] !== undefined
}

function restoreAll() {
  resetAllShortcuts()
  showToast("已恢复全部默认快捷键", "success")
}

onUnmounted(stopListening)
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

    <SettingSection title="系统通知">
      <SettingCard
        title="Windows 系统通知"
        description="构建完成、批量配置、记录/导入完成、批量删除等重要结果，额外推送一条系统通知；应用内提示始终显示，普通操作不会打扰"
      >
        <SettingSwitch v-model="settings.systemNotify" />
      </SettingCard>
    </SettingSection>

    <SettingSection title="快捷键">
      <div
        class="flex w-full flex-col rounded-xl border border-transparent bg-muted/40 px-3 py-2.5"
      >
        <div class="mb-2.5 flex items-center justify-between gap-3">
          <p
            class="text-muted-foreground text-[clamp(10px,1.1vw,11px)] leading-relaxed"
          >
            点击「修改」后按下新的组合键；Esc 取消录入，Backspace
            清除绑定；与其它快捷键冲突时会被拒绝
          </p>
          <button
            type="button"
            class="hover:bg-muted text-muted-foreground hover:text-foreground inline-flex shrink-0 cursor-pointer items-center gap-1 rounded-md bg-muted/60 px-2 py-1 text-[clamp(10px,1.1vw,11px)] font-medium transition-colors duration-200 focus-visible:outline-none"
            title="清除全部自定义，恢复默认快捷键"
            @click="restoreAll"
          >
            <RotateCcw class="size-2.5" />
            全部恢复默认
          </button>
        </div>
        <div class="flex flex-col gap-1.5">
          <div
            v-for="action in SHORTCUT_ORDER"
            :key="action"
            class="bg-background/60 flex items-center gap-2 rounded-lg border border-border/60 px-2.5 py-1.5 transition-colors duration-200 hover:border-border"
            :class="listening === action ? 'border-primary/40 bg-primary/5' : ''"
          >
            <span
              class="min-w-0 flex-1 truncate text-[clamp(11px,1.25vw,12px)] font-medium"
            >
              {{ SHORTCUT_LABELS[action] ?? action }}
            </span>
            <!-- Key cap: primary-tinted chip, same family as the template
                 name tags used across the app. -->
            <kbd
              class="bg-primary/10 text-primary shrink-0 rounded-md px-2 py-0.5 font-mono text-[clamp(9px,1vw,10px)] font-medium"
              :class="listening === action ? 'ring-primary/40 ring-2' : ''"
            >
              {{
                listening === action
                  ? "按下按键…"
                  : formatCombo(shortcutFor(action))
              }}
            </kbd>
            <button
              type="button"
              class="text-muted-foreground hover:text-foreground hover:bg-muted inline-flex shrink-0 cursor-pointer items-center rounded-md px-1.5 py-0.5 text-[clamp(9px,1vw,10px)] font-medium transition-colors duration-200 focus-visible:outline-none"
              :class="listening === action ? 'text-primary font-semibold' : ''"
              @click="startListening(action)"
            >
              {{ listening === action ? "录入中…" : "修改" }}
            </button>
            <button
              v-if="isCustomized(action)"
              type="button"
              class="text-muted-foreground hover:text-foreground hover:bg-muted inline-flex shrink-0 cursor-pointer items-center rounded-md p-0.5 transition-colors duration-200 focus-visible:outline-none"
              aria-label="恢复该快捷键默认值"
              title="恢复默认"
              @click="resetShortcut(action)"
            >
              <RotateCcw class="size-3" />
            </button>
          </div>
        </div>
      </div>
    </SettingSection>
  </div>
</template>
