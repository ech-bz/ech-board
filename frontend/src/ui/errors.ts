export function reportError(e: unknown): void {
  window.alert(e instanceof Error ? e.message : String(e))
}
