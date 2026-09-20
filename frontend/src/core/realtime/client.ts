import type { Batch } from '../events/batch'

export interface RealtimeHandlers {
  onReload: () => void
  onDrop: () => void
  onRestored: () => void
}

export class RealtimeClient {
  private es: EventSource | null = null
  private url: string | null = null
  private scope: string | null = null
  private pending: Batch[] = []
  private buffering = false
  private live: ((msg: Batch) => void) | null = null
  private pendingCommit = false
  private everConnected = false
  private handlers: RealtimeHandlers

  constructor(handlers: RealtimeHandlers) {
    this.handlers = handlers
  }

  begin(url: string, scope: string): void {
    const changed = this.url !== url || this.scope !== scope
    this.url = url
    this.scope = scope
    this.pendingCommit = true
    this.buffering = true
    this.live = null
    this.pending = []
    if (changed || !this.es) this.connect()
  }

  commit(apply: (msg: Batch) => void): void {
    this.live = apply
    this.buffering = false
    const pending = this.pending
    this.pending = []
    for (const msg of pending) apply(msg)
    this.pendingCommit = false
  }

  close(): void {
    this.es?.close()
    this.es = null
  }

  private connect(): void {
    const url = this.url
    const scope = this.scope
    if (!url || !scope) return
    this.close()
    const es = new EventSource(`${url}/sse/${scope}`)
    this.es = es
    es.onopen = () => {
      this.everConnected = true
      this.handlers.onRestored()
      if (this.pendingCommit) return
      this.buffering = true
      this.live = null
      this.pending = []
      this.handlers.onReload()
    }
    es.onmessage = (e) => {
      let msg: Batch
      try {
        msg = JSON.parse(e.data)
      } catch {
        return
      }
      if (this.buffering) this.pending.push(msg)
      else this.live?.(msg)
    }
    es.onerror = () => {
      this.pendingCommit = false
      if (this.everConnected) this.handlers.onDrop()
    }
  }
}
