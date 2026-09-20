import { PostProjection, type MediaMetaData, type PostObject } from './bcs/types'
import { toHex } from './intent/crypto'

export interface MediaInfo {
  thumbUrl: string
  fullUrl: string
  width: number
  height: number
  mime: string
  size: number
  durationMs: number | null
  postUid: string
  threadUid: string
  hash: string
  banned: boolean
}

export interface ReactionInfo {
  url: string
  count: number
  hex: string
}

export function contentUrl(relayUrl: string, boardUid: string, threadUid: string, postUid: string, kind: string, hex: string): string {
  return `${relayUrl}/content/${boardUid}/${threadUid}/${postUid}/${kind}/${hex}`
}

export function reactionUrl(relayUrl: string, boardUid: string, hex: string): string {
  return `${relayUrl}/content/${boardUid}/reaction/${hex}`
}

const mediaCache = new Map<string, MediaInfo[]>()

function sameMedia(a: readonly MediaInfo[], b: readonly MediaInfo[]): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) {
    const x = a[i]
    const y = b[i]
    if (x === y) continue
    if (x.thumbUrl !== y.thumbUrl || x.fullUrl !== y.fullUrl || x.width !== y.width || x.height !== y.height || x.mime !== y.mime || x.size !== y.size || x.durationMs !== y.durationMs || x.hash !== y.hash || x.banned !== y.banned || x.postUid !== y.postUid || x.threadUid !== y.threadUid) return false
  }
  return true
}

export function postMedia(post: PostObject, relayUrl: string, boardUid: string, threadUid: string, mediaMeta: Map<string, MediaMetaData>, showBanned = false): MediaInfo[] {
  const postUid = toHex(post.root.id)
  const pp = new PostProjection(post.projection)
  const banned = new Set(pp.banned_media().map(toHex))
  const media: MediaInfo[] = []
  for (const h of pp.media_hashes()) {
    const hex = toHex(h)
    const isBanned = banned.has(hex)
    if (isBanned && !showBanned) continue
    const meta = mediaMeta.get(hex)
    if (!meta) continue
    media.push({
      thumbUrl: contentUrl(relayUrl, boardUid, threadUid, postUid, 'thumbnail', hex),
      fullUrl: contentUrl(relayUrl, boardUid, threadUid, postUid, 'media', hex),
      width: meta.width,
      height: meta.height,
      mime: meta.mime,
      size: Number(meta.size),
      durationMs: meta.duration_ms != null ? Number(meta.duration_ms) : null,
      postUid,
      threadUid,
      hash: hex,
      banned: isBanned,
    })
  }
  const key = `${relayUrl}|${boardUid}|${threadUid}|${postUid}|${showBanned ? '1' : '0'}`
  const hit = mediaCache.get(key)
  if (hit && sameMedia(hit, media)) return hit
  mediaCache.set(key, media)
  return media
}

export function clearMediaCache(): void {
  mediaCache.clear()
}

export function postReactions(post: PostObject, relayUrl: string, boardUid: string): ReactionInfo[] {
  const pp = new PostProjection(post.projection)
  return pp.reactions().map(([h, c]: [Uint8Array, string]) => {
    const hex = toHex(h)
    return { url: reactionUrl(relayUrl, boardUid, hex), count: Number(c), hex }
  })
}
