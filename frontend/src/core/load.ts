import { ensureDefaultRelay, ensureDefaultForumId, resolveRelayUrl } from './config'
import { getBoardReactions, getForumInfo } from './api/cache'
import { fetchMyReactions } from './api/myReactions'
import { fetchBoardView, fetchThreadView, fetchPostView } from './api/relay'
import { BoardView, ThreadView, PostView, BoardProjection, type BoardObject, type ThreadObject, type PostObject, type MediaMetaData, type ModeratorsData } from './bcs/types'
import { toHex, fromHex } from './intent/crypto'
import { getSecretKeyBytes } from './keys/storage'
import { buildDecryptContext, type DecryptContext } from './intent/decrypt'
import type { PostRef } from './posts'

export interface PostCtx {
  relayUrl: string
  forumId: string
  boardUid: string
  nonceShardsId: string
  moderators: ModeratorsData
  contentMap: Map<string, Uint8Array>
  mediaMeta: Map<string, MediaMetaData>
  decryptCtx: DecryptContext
  boardReactions: Uint8Array[]
  myReactions: Map<string, string>
}

export interface BoardViewData extends PostCtx {
  board: BoardObject
  threads: ThreadObject[]
  opPosts: Map<string, PostObject>
  last3: Map<string, PostObject[]>
  nextCursor: number | null
}

export interface ThreadViewData extends PostCtx {
  threadUid: string
  thread: ThreadObject
  posts: PostObject[]
}

export async function ensureDefaultContext(): Promise<{ forumId: string; relayUrl: string }> {
  await ensureDefaultRelay()
  const forumId = await ensureDefaultForumId()
  if (!forumId) throw new Error('default forum id missing')
  const relayUrl = resolveRelayUrl(forumId)
  if (!relayUrl) throw new Error('relay not found for default forum')
  return { forumId, relayUrl }
}

export function contentMapOf(entries: [Uint8Array, number[]][]): Map<string, Uint8Array> {
  const map = new Map<string, Uint8Array>()
  for (const [k, v] of entries) map.set(toHex(k), new Uint8Array(v))
  return map
}

export function mediaMetaMapOf(entries: [Uint8Array, MediaMetaData][]): Map<string, MediaMetaData> {
  const map = new Map<string, MediaMetaData>()
  for (const [k, v] of entries) map.set(toHex(k), v)
  return map
}

export async function loadBoardViewData(relayUrl: string, boardUid: string): Promise<BoardViewData> {
  const bytes = await fetchBoardView(relayUrl, boardUid)
  const view = BoardView.parse(new Uint8Array(bytes))
  const contentMap = contentMapOf([...view.text, ...view.plain_text])
  const mediaMeta = mediaMetaMapOf(view.media_meta)
  const opPosts = new Map<string, PostObject>()
  for (const [k, v] of view.op_posts) opPosts.set(toHex(k), v)
  const last3 = new Map<string, PostObject[]>()
  for (const [k, v] of view.last_3) last3.set(toHex(k), v)
  const forum = await getForumInfo(relayUrl)
  const forumId = forum.forumId.replace(/^0x/, '')
  const posts = [...opPosts.values(), ...Array.from(last3.values()).flat()]
  const decryptCtx = buildDecryptContext(posts, getSecretKeyBytes(), fromHex(forumId))
  const myReactions = await fetchMyReactions(relayUrl, posts, forumId)
  return { relayUrl, forumId, boardUid, nonceShardsId: forum.nonceShardsId, moderators: view.moderators, board: view.board, threads: view.threads, opPosts, last3, nextCursor: view.next_cursor != null ? Number(view.next_cursor) : null, contentMap, mediaMeta, decryptCtx, boardReactions: getBoardReactions(relayUrl, boardUid), myReactions }
}

export interface BoardPageData {
  threads: ThreadObject[]
  opPosts: Map<string, PostObject>
  last3: Map<string, PostObject[]>
  contentMap: Map<string, Uint8Array>
  mediaMeta: Map<string, MediaMetaData>
  myReactions: Map<string, string>
  nextCursor: number | null
}

export async function loadBoardPage(relayUrl: string, boardUid: string, cursor: number): Promise<BoardPageData> {
  const bytes = await fetchBoardView(relayUrl, boardUid, cursor)
  const view = BoardView.parse(new Uint8Array(bytes))
  const opPosts = new Map<string, PostObject>()
  for (const [k, v] of view.op_posts) opPosts.set(toHex(k), v)
  const last3 = new Map<string, PostObject[]>()
  for (const [k, v] of view.last_3) last3.set(toHex(k), v)
  const forum = await getForumInfo(relayUrl)
  const myReactions = await fetchMyReactions(relayUrl, [...opPosts.values(), ...Array.from(last3.values()).flat()], forum.forumId.replace(/^0x/, ''))
  return {
    threads: view.threads,
    opPosts,
    last3,
    contentMap: contentMapOf([...view.text, ...view.plain_text]),
    mediaMeta: mediaMetaMapOf(view.media_meta),
    myReactions,
    nextCursor: view.next_cursor != null ? Number(view.next_cursor) : null,
  }
}

export async function loadThreadViewData(relayUrl: string, boardUid: string, threadUid: string): Promise<ThreadViewData> {
  const bytes = await fetchThreadView(relayUrl, threadUid)
  const view = ThreadView.parse(new Uint8Array(bytes))
  const contentMap = contentMapOf([...view.text, ...view.plain_text])
  const mediaMeta = mediaMetaMapOf(view.media_meta)
  const forum = await getForumInfo(relayUrl)
  const forumId = forum.forumId.replace(/^0x/, '')
  const decryptCtx = buildDecryptContext(view.posts, getSecretKeyBytes(), fromHex(forumId))
  const myReactions = await fetchMyReactions(relayUrl, view.posts, forumId)
  return { relayUrl, forumId, boardUid, nonceShardsId: forum.nonceShardsId, moderators: view.moderators, threadUid, thread: view.thread, posts: view.posts, contentMap, mediaMeta, decryptCtx, boardReactions: getBoardReactions(relayUrl, boardUid), myReactions }
}

export interface TooltipPost {
  post: PostObject
  threadUid: string
  ctx: PostCtx
  threadObj?: ThreadObject
  slug: string
  refs?: PostRef[]
}

export async function loadPostTooltip(relayUrl: string, forumId: string, postUid: string): Promise<TooltipPost> {
  const bytes = await fetchPostView(relayUrl, postUid)
  const view = PostView.parse(new Uint8Array(bytes))
  const threadUid = toHex(view.thread.root.id)
  const boardUid = toHex(view.board.root.id)
  const contentMap = contentMapOf([...view.text, ...view.plain_text])
  const mediaMeta = mediaMetaMapOf(view.media_meta)
  const decryptCtx = buildDecryptContext([view.post], getSecretKeyBytes(), fromHex(forumId))
  const forum = await getForumInfo(relayUrl)
  const myReactions = await fetchMyReactions(relayUrl, [view.post], forumId)
  const ctx: PostCtx = {
    relayUrl,
    forumId,
    boardUid,
    nonceShardsId: forum.nonceShardsId,
    moderators: view.moderators,
    contentMap,
    mediaMeta,
    decryptCtx,
    boardReactions: getBoardReactions(relayUrl, boardUid),
    myReactions,
  }
  return { post: view.post, threadUid, ctx, threadObj: view.thread, slug: new BoardProjection(view.board.projection).slug() }
}
