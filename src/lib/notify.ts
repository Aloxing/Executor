import { settings } from "./settings"

/**
 * Windows 系统通知（重要事件专用）。
 *
 * 应用内的 toast 始终显示；这里只负责把**耗时或关键**的结果（构建完成、
 * 批量配置、记录/导入完成、批量删除…）再推一条系统通知，这样用户切到别的
 * 窗口或把应用收到托盘时也不会错过。普通的增删改提示不走这里，避免右下角
 * 被连续打扰。
 *
 * 所有失败一律静默：通知只是锦上添花，绝不能影响业务操作本身（与后端
 * `records::log_operation` 的容错原则一致）。
 */

// 权限每次会话只问一次；Windows 上通常直接就是 granted。
let permitted: boolean | null = null

/**
 * Raises one system notification. `title` names the event (构建完成 /
 * 批量配置完成…) and `body` carries the detail, mirroring the toast text.
 */
export async function notifySystem(
  title: string,
  body: string
): Promise<void> {
  if (!settings.systemNotify) return
  if (!body.trim()) return
  try {
    const {
      isPermissionGranted,
      requestPermission,
      sendNotification,
    } = await import("@tauri-apps/plugin-notification")
    if (permitted === null) {
      permitted = await isPermissionGranted()
      if (!permitted) {
        permitted = (await requestPermission()) === "granted"
      }
    }
    if (!permitted) return
    sendNotification({ title, body })
  } catch {
    // 不在 Tauri 环境 / 插件不可用 / Windows 通知被系统关闭：应用内 toast
    // 已经把消息传达给用户了。
  }
}
