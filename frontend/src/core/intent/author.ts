import { fetchBanHashes } from '../api/relay'
import { concatBytes, fromHex, sign, toHex } from './crypto'

export interface UidCandidate {
  id: string
  uid: Uint8Array
}

const EXACT_IP_MASK_INDEX = 0
const MATCH_CONCURRENCY = 6
const CACHE_LIMIT = 5000

const keyCache = new Map<string, string>()

function cacheKey(scope: string, uid: Uint8Array): string {
  return `${scope}\n${toHex(uid)}`
}

function cachePut(key: string, value: string): void {
  if (keyCache.size >= CACHE_LIMIT) {
    const oldest = keyCache.keys().next().value
    if (oldest !== undefined) keyCache.delete(oldest)
  }
  keyCache.set(key, value)
}

async function uidKey(relayUrl: string, uid: Uint8Array, pathHex: string[], postHex: string, pk: Uint8Array, secretKey: Uint8Array): Promise<string> {
  const path = [...pathHex, postHex]
  const signature = await sign(secretKey, concatBytes(uid, ...path.map(fromHex), pk))
  const hashes = await fetchBanHashes(relayUrl, uid, path, pk, signature)
  return toHex(hashes[EXACT_IP_MASK_INDEX])
}

export async function matchPostsByAuthor(opts: {
  relayUrl: string
  pathHex: string[]
  pk: Uint8Array
  secretKey: Uint8Array
  refId: string
  candidates: UidCandidate[]
  onProgress?: (done: number, total: number) => void
}): Promise<string[]> {
  const scope = opts.pathHex.join(':')
  const queue = opts.candidates.filter((c) => c.uid.length > 0)
  const keys = new Map<string, string>()
  const pending: UidCandidate[] = []
  for (const item of queue) {
    const cached = keyCache.get(cacheKey(scope, item.uid))
    if (cached === undefined) pending.push(item)
    else keys.set(item.id, cached)
  }
  let cursor = 0
  let done = queue.length - pending.length
  opts.onProgress?.(done, queue.length)
  const worker = async () => {
    while (cursor < pending.length) {
      const item = pending[cursor++]
      const key = await uidKey(opts.relayUrl, item.uid, opts.pathHex, item.id, opts.pk, opts.secretKey)
      cachePut(cacheKey(scope, item.uid), key)
      keys.set(item.id, key)
      done += 1
      opts.onProgress?.(done, queue.length)
    }
  }
  await Promise.all(Array.from({ length: Math.min(MATCH_CONCURRENCY, pending.length) }, worker))
  const refKey = keys.get(opts.refId)
  if (refKey === undefined) throw new Error('author match: no uid for the selected post')
  return opts.candidates.filter((c) => keys.get(c.id) === refKey).map((c) => c.id)
}
