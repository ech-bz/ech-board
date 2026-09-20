interface Entry {
  at: number
  fire: () => void
}

const MAX_WAIT_MS = 60000

export class DeadlineScheduler {
  private entries = new Map<string, Entry>()
  private timer: ReturnType<typeof setTimeout> | null = null

  schedule(id: string, at: number, fire: () => void): void {
    this.entries.set(id, { at, fire })
    this.rearm()
  }

  cancel(id: string): void {
    if (!this.entries.delete(id)) return
    this.rearm()
  }

  dispose(): void {
    this.entries.clear()
    this.disarm()
  }

  private fire(): void {
    this.timer = null
    const now = Date.now()
    const due: (() => void)[] = []
    for (const [id, entry] of this.entries) {
      if (entry.at <= now) {
        due.push(entry.fire)
        this.entries.delete(id)
      }
    }
    for (const fn of due) fn()
    this.rearm()
  }

  private rearm(): void {
    this.disarm()
    if (this.entries.size === 0) return
    let earliest = Infinity
    for (const entry of this.entries.values()) if (entry.at < earliest) earliest = entry.at
    const wait = Math.max(0, Math.min(earliest - Date.now(), MAX_WAIT_MS))
    this.timer = setTimeout(() => this.fire(), wait)
  }

  private disarm(): void {
    if (this.timer === null) return
    clearTimeout(this.timer)
    this.timer = null
  }
}
