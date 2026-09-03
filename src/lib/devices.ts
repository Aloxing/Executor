/** Android device USB-debug support: detect devices through adb and
 * stream their logcat into the build log area (one tab per device). */

export interface AndroidDevice {
  serial: string
  /** Parsed from `model:` (underscores turned into spaces). */
  model: string
  product: string
  /** `device` (authorized), `unauthorized` or `offline`. */
  status: string
}

/** One streamed logcat chunk of the `device-log` event; `line` may
 * contain several `\n`-joined lines (the backend coalesces output). */
export interface DeviceLogEvent {
  serial: string
  /** `status` (flow markers), `stdout` (logcat output) or `done`. */
  kind: "status" | "stdout" | "done"
  line: string
  success?: boolean
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core")
  return invoke<T>(cmd, args)
}

/** Lists USB-debug devices via `adb devices -l`. */
export async function listAndroidDevices(): Promise<AndroidDevice[]> {
  return invoke<AndroidDevice[]>("list_android_devices")
}

/** Installs one apk onto the device (`adb install -r`, keeps app data).
 * Resolves to a user-facing Chinese summary; may take tens of seconds. */
export async function installApk(path: string, serial: string): Promise<string> {
  return invoke<string>("install_apk", { path, serial })
}

/** Starts streaming one device's logcat; with a non-empty `packageName`
 * only that app's logs are captured (auto-attach / re-attach by pid).
 * The promise stays pending until the capture is stopped. */
export async function startDeviceLogcat(
  serial: string,
  packageName?: string
): Promise<void> {
  return invoke("start_device_logcat", {
    serial,
    packageName: packageName ?? "",
  })
}

/** Stops the logcat capture of one device (kills the adb client). */
export async function stopDeviceLogcat(serial: string): Promise<void> {
  return invoke("stop_device_logcat", { serial })
}

/** Subscribes to the streamed device logs; resolves to the unlisten fn. */
export async function listenDeviceLog(
  handler: (event: DeviceLogEvent) => void
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event")
  return listen<DeviceLogEvent>("device-log", (event) => handler(event.payload))
}

/** Log-page id of a device inside the build log area. */
export function deviceLogId(serial: string): string {
  return `device:${serial}`
}
