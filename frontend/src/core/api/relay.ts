import { ForumView, NonceInfo, SendResponse, ReactionsBatch } from '../bcs/types'
import { toHex } from '../intent/crypto'

export async function fetchForum(relayUrl: string) {
  const resp = await fetch(relayUrl + '/forum')
  if (!resp.ok) throw new Error(`forum: HTTP ${resp.status}`)
  const bytes = new Uint8Array(await resp.arrayBuffer())
  const parsed = ForumView.parse(bytes)
  return parsed
}

export async function fetchBoardView(relayUrl: string, uid: string, cursor?: number): Promise<ArrayBuffer> {
  const params = cursor ? `?cursor=${cursor}` : ''
  const resp = await fetch(`${relayUrl}/board/${uid}${params}`)
  if (!resp.ok) throw new Error(`board: HTTP ${resp.status}`)
  return resp.arrayBuffer()
}

export async function fetchThreadView(relayUrl: string, uid: string): Promise<ArrayBuffer> {
  const resp = await fetch(`${relayUrl}/thread/${uid}`)
  if (!resp.ok) throw new Error(`thread: HTTP ${resp.status}`)
  return resp.arrayBuffer()
}

export async function fetchPostView(relayUrl: string, uid: string): Promise<ArrayBuffer> {
  const resp = await fetch(`${relayUrl}/post/${uid}`)
  if (!resp.ok) throw new Error(`post: HTTP ${resp.status}`)
  return resp.arrayBuffer()
}

export async function fetchReactions(relayUrl: string, pairs: [Uint8Array, Uint8Array][]): Promise<[string, string][]> {
  const body = ReactionsBatch.serialize(pairs).toBytes()
  const resp = await fetch(`${relayUrl}/reactions`, { method: 'POST', body })
  if (!resp.ok) throw new Error(`reactions batch: HTTP ${resp.status}`)
  const parsed = ReactionsBatch.parse(new Uint8Array(await resp.arrayBuffer()))
  return parsed.map(([p, r]: [Uint8Array, Uint8Array]) => [toHex(p), toHex(r)])
}

export async function fetchResolvePost(relayUrl: string, boardUid: string, number: number): Promise<string | null> {
  const resp = await fetch(`${relayUrl}/board/${boardUid}/post/${number}`)
  if (!resp.ok) return null
  return toHex(new Uint8Array(await resp.arrayBuffer()))
}

export async function fetchResolveThread(relayUrl: string, boardUid: string, number: number): Promise<string | null> {
  const resp = await fetch(`${relayUrl}/board/${boardUid}/thread/${number}`)
  if (!resp.ok) return null
  return toHex(new Uint8Array(await resp.arrayBuffer()))
}

export async function fetchFeed(relayUrl: string, uid: string, counter: number, cursor?: number): Promise<ArrayBuffer> {
  const params = new URLSearchParams()
  params.set('counter', String(counter))
  if (cursor != null) params.set('cursor', String(cursor))
  const resp = await fetch(`${relayUrl}/feed/${uid}?${params}`)
  if (!resp.ok) throw new Error(`feed: HTTP ${resp.status}`)
  return resp.arrayBuffer()
}

export async function fetchBans(relayUrl: string, uid: string, cursor?: number): Promise<ArrayBuffer> {
  const params = cursor != null ? `?cursor=${cursor}` : ''
  const resp = await fetch(`${relayUrl}/bans/${uid}${params}`)
  if (!resp.ok) throw new Error(`bans: HTTP ${resp.status}`)
  return resp.arrayBuffer()
}

export async function fetchNonce(relayUrl: string, sender: string): Promise<number> {
  const resp = await fetch(`${relayUrl}/nonce/${sender}`)
  if (!resp.ok) throw new Error(`nonce: HTTP ${resp.status}`)
  const bytes = new Uint8Array(await resp.arrayBuffer())
  const info = NonceInfo.parse(bytes)
  return Number(info.nonce)
}

export interface SendItem {
  intentBytes: Uint8Array
  signature: Uint8Array
  text?: Uint8Array
  media?: Uint8Array[]
  description?: string
  topic?: string
  name?: string
  tripcode?: string
}

export interface SendOpts {
  reason?: string
  onProgress?: (loaded: number, total: number) => void
  signal?: AbortSignal
}

export async function sendIntents(relayUrl: string, items: SendItem[], opts?: SendOpts) {
  const form = new FormData()
  for (const it of items) {
    form.append('intent', new Blob([it.intentBytes], { type: 'application/octet-stream' }), 'intent.bin')
    form.append('signature', new Blob([it.signature], { type: 'application/octet-stream' }), 'sig.bin')
    if (it.text) {
      form.append('text', new Blob([it.text], { type: 'application/octet-stream' }), 'text.bin')
    }
    if (it.media) {
      for (const blob of it.media) {
        form.append('media', new Blob([blob], { type: 'application/octet-stream' }), 'media.bin')
      }
    }
    if (it.description) {
      form.append('description', it.description)
    }
    if (it.topic) {
      form.append('topic', it.topic)
    }
    if (it.name != null) {
      form.append('name', it.name)
    }
    if (it.tripcode != null) {
      form.append('tripcode', it.tripcode)
    }
  }
  form.append('captcha', 'test')
  if (opts?.reason != null) {
    form.append('reason', opts.reason)
  }

  const resp = await new Promise<{ status: number; body: ArrayBuffer }>((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', `${relayUrl}/send`)
    xhr.responseType = 'arraybuffer'
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable && opts?.onProgress) opts.onProgress(e.loaded, e.total)
    }
    xhr.onload = () => resolve({ status: xhr.status, body: xhr.response as ArrayBuffer })
    xhr.onerror = () => reject(new Error('network error'))
    xhr.onabort = () => reject(new DOMException('Aborted', 'AbortError'))
    if (opts?.signal) {
      if (opts.signal.aborted) {
        reject(new DOMException('Aborted', 'AbortError'))
        return
      }
      opts.signal.addEventListener('abort', () => xhr.abort(), { once: true })
    }
    xhr.send(form)
  })
  if (resp.status < 200 || resp.status >= 300) {
    const err = new TextDecoder().decode(resp.body)
    throw new Error(err || `send: HTTP ${resp.status}`)
  }
  return SendResponse.parse(new Uint8Array(resp.body))
}

export async function fetchBanHashes(
  relayUrl: string,
  uid: Uint8Array,
  path: string[],
  pk: Uint8Array,
  signature: Uint8Array,
): Promise<Uint8Array[]> {
  const resp = await fetch(`${relayUrl}/decrypt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      uid: Array.from(uid),
      path,
      pk: toHex(pk),
      signature: Array.from(signature),
    }),
  })
  if (!resp.ok) {
    const err = await resp.text().catch(() => '')
    throw new Error(err || `decrypt: HTTP ${resp.status}`)
  }
  const bytes = new Uint8Array(await resp.arrayBuffer())
  const hashes: Uint8Array[] = []
  for (let i = 0; i < 4; i++) {
    hashes.push(bytes.slice(i * 32, (i + 1) * 32))
  }
  return hashes
}
