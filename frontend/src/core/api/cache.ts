import { fetchForum, fetchResolveThread } from './relay'
import { ForumView } from '../bcs/relay'
import type { InferBcsType } from '@mysten/bcs'
import { toHex } from '../intent/crypto'
import { BoardProjection, ForumProjection, type BoardObject, type ModeratorsData } from '../bcs/types'

let forumCache: { forumId: string; nonceShardsId: string; boardBySlug: Map<string, string>; boardDescriptions: Map<string, string>; boardReactions: Map<string, Uint8Array[]>; boardMaxMedia: Map<string, number>; boardPostCounts: Map<string, number>; boardDeleted: Set<string>; moderators: ModeratorsData; relayUrl: string } | null = null
let boardThreadsCache: Map<string, Map<number, string>> = new Map()

export function warmForumCache(forum: InferBcsType<typeof ForumView>, relayUrl: string) {
  const plainMap = new Map<string, string>()
  for (const [k, v] of forum.plain_text) {
    plainMap.set(toHex(k), new TextDecoder().decode(new Uint8Array(v)))
  }
  const boardDescriptions = new Map<string, string>()
  const boardReactions = new Map<string, Uint8Array[]>()
  const boardMaxMedia = new Map<string, number>()
  const boardPostCounts = new Map<string, number>()
  const boardDeleted = new Set<string>()
  for (const b of forum.boards) {
    const bp = new BoardProjection(b.projection)
    if (!!bp.deleted()) boardDeleted.add(bp.slug())
    const descHash = bp.description_hash()
    boardDescriptions.set(
      bp.slug(),
      descHash ? plainMap.get(toHex(descHash)) ?? '' : '',
    )
    boardReactions.set(
      bp.slug(),
      bp.reactions().map((r: Uint8Array) => new Uint8Array(r)),
    )
    boardMaxMedia.set(bp.slug(), Number(bp.max_media()))
    boardPostCounts.set(bp.slug(), Number(bp.posts().size))
  }
  forumCache = {
    forumId: toHex(forum.forum.root.id),
    nonceShardsId: toHex(new ForumProjection(forum.forum.projection).nonce_shards()),
    boardBySlug: new Map(forum.boards.map((b: BoardObject) => [new BoardProjection(b.projection).slug(), toHex(b.root.id)])),
    boardDescriptions,
    boardReactions,
    boardMaxMedia,
    boardPostCounts,
    boardDeleted,
    moderators: forum.moderators,
    relayUrl,
  }
}

export async function getForumInfo(relayUrl: string) {
  if (forumCache && forumCache.relayUrl === relayUrl) return forumCache
  const forum = await fetchForum(relayUrl)
  warmForumCache(forum, relayUrl)
  return forumCache!
}

export function getForumModerators(relayUrl: string): ModeratorsData | null {
  return forumCache && forumCache.relayUrl === relayUrl ? forumCache.moderators : null
}

export function getBoardReactions(relayUrl: string, boardUid: string): Uint8Array[] {
  if (!forumCache || forumCache.relayUrl !== relayUrl) return []
  for (const [slug, uid] of forumCache.boardBySlug) {
    if (uid === boardUid) return forumCache.boardReactions.get(slug) ?? []
  }
  return []
}

export async function getThreadUid(relayUrl: string, slug: string, threadNum: number) {
  const forum = await getForumInfo(relayUrl)
  const boardUid = forum.boardBySlug.get(slug)
  if (!boardUid) throw new Error(`board not found: ${slug}`)

  const existing = boardThreadsCache.get(boardUid)?.get(threadNum)
  if (existing) return { boardUid, threadUid: existing }

  const threadUid = await fetchResolveThread(relayUrl, boardUid, threadNum)
  if (!threadUid) throw new Error(`thread not found: ${threadNum}`)
  const map = boardThreadsCache.get(boardUid) ?? new Map()
  map.set(threadNum, threadUid)
  boardThreadsCache.set(boardUid, map)
  return { boardUid, threadUid }
}
