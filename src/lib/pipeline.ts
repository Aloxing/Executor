import { ref } from "vue"

/** Pipeline handoff between pages.
 *
 * `pendingBuildRequest` holds the build-queue uuid that the config area
 * forwarded for building; the build page consumes it once (opening the
 * build-mode dialog for that queue) and clears it. */
export const pendingBuildRequest = ref<string | null>(null)
