import { BoardProjection, type ThreadProjectionData, type ThreadObject, type PostObject } from '../bcs/types'
import type { BoardViewData, ThreadViewData } from '../load'
import { decodeRealtimeEvent, type DecodedEvent } from './core'
import { applyPost, emptyPostProjection } from './post_apply'
import { applyThread, applyPostFlags, adjustPostsDeleted, postsDeletedDelta, emptyThreadProjection, applyThreadGenesis } from './thread_apply'
import { addPostsToDecryptContext } from '../intent/decrypt'
import { fromHex, toHex } from '../intent/crypto'

const BOARD_PAGE_THREADS = 20

export interface EntityRoot {
  id: string
  entity: { feed: { id: string; counter: number }; version: number }
  genesis: boolean
}

export interface Envelope {
  kind: 'forum' | 'board' | 'thread' | 'post'
  target: EntityRoot
  event: string
}

export interface Batch {
  forum: string | null
  board: string | null
  thread: string | null
  post: string | null
  envelopes: Envelope[]
}

export function snapshotBoard(d: BoardViewData): Map<string, number> {
  const m = new Map<string, number>()
  for (const t of d.threads) m.set(toHex(t.root.id), Number(t.root.entity.feed.counter))
  for (const p of d.opPosts.values()) m.set(toHex(p.root.id), Number(p.root.entity.feed.counter))
  for (const list of d.last3.values()) for (const p of list) m.set(toHex(p.root.id), Number(p.root.entity.feed.counter))
  return m
}

export function snapshotThread(d: ThreadViewData): Map<string, number> {
  const m = new Map<string, number>()
  m.set(toHex(d.thread.root.id), Number(d.thread.root.entity.feed.counter))
  for (const p of d.posts) m.set(toHex(p.root.id), Number(p.root.entity.feed.counter))
  return m
}

export interface ThreadBatchResult {
  data: ThreadViewData
}

export function applyThreadBatch(d: ThreadViewData, batch: Batch, applied: Map<string, number>): ThreadBatchResult {
  let data = d
  let postIndex: Map<string, number> | null = null
  const index = () => {
    if (!postIndex) {
      const m = new Map<string, number>()
      for (let i = 0; i < data.posts.length; i++) m.set(toHex(data.posts[i].root.id), i)
      postIndex = m
    }
    return postIndex
  }
  const postById = (id: string): PostObject | undefined => {
    const i = index().get(id)
    return i === undefined ? undefined : data.posts[i]
  }
  const replacePost = (id: string, next: PostObject) => {
    const i = index().get(id)
    if (i === undefined) return
    const posts = data.posts.slice()
    posts[i] = next
    data = { ...data, posts }
  }
  for (const env of batch.envelopes) {
    const target = env.target
    const counter = target.entity.feed.counter ?? 0
    if (counter <= (applied.get(target.id) ?? 0)) continue
    applied.set(target.id, counter)
    if (!env.event || !env.kind) continue
    let ev: DecodedEvent
    try { ev = decodeRealtimeEvent(env.kind, env.event) } catch { continue }

    if (env.kind === 'post') {
      if (ev.$kind === 'genesis' || ev.$kind === 'genesis_v2') {
        if (index().has(target.id)) continue
        const post = {
          root: { id: fromHex(target.id), entity: { feed: { id: fromHex(target.entity.feed.id), counter: String(counter) }, version: target.entity.version }, genesis: false },
          projection: emptyPostProjection(target.entity.version),
        }
        const appliedPost = applyPost(post, ev, target.entity.version)
        data = { ...data, posts: [...data.posts, appliedPost], decryptCtx: addPostsToDecryptContext(data.decryptCtx, [appliedPost]) }
        index().set(target.id, data.posts.length - 1)
      } else {
        const cur = postById(target.id)
        if (cur) replacePost(target.id, applyPost(cur, ev, target.entity.version))
      }
      continue
    }

    if (env.kind === 'thread' && target.id === d.threadUid) {
      const postId = batch.post
      switch (ev.$kind) {
        case 'add_moderator':
        case 'del_moderator':
          break
        case 'post_set_deleted':
        case 'post_set_text':
        case 'ban':
        case 'unban': {
          if (!postId) break
          const flagged = postById(postId)
          if (flagged) replacePost(postId, applyPostFlags(flagged, ev))
          if (ev.$kind === 'post_set_deleted' || ev.$kind === 'post_set_text') {
            const curPost = postById(postId)
            const delta = curPost ? postsDeletedDelta(curPost, ev) : 0
            if (delta !== 0) data = { ...data, thread: adjustPostsDeleted(data.thread, delta) }
          }
          break
        }
        default:
          data = { ...data, thread: applyThread(data.thread, ev, target.entity.version) }
      }
    }
  }
  return { data }
}

export interface BoardBatchResult {
  data: BoardViewData
}

export function applyBoardBatch(d: BoardViewData, batch: Batch, applied: Map<string, number>): BoardBatchResult {
  let data = d
  const pendingThreads = new Map<string, ThreadObject>()
  for (const env of batch.envelopes) {
    const target = env.target
    const counter = target.entity.feed.counter ?? 0
    if (counter <= (applied.get(target.id) ?? 0)) continue
    applied.set(target.id, counter)
    if (!env.event || !env.kind) continue
    let ev: DecodedEvent
    try { ev = decodeRealtimeEvent(env.kind, env.event) } catch { continue }

    if (env.kind === 'thread') {
      if (ev.$kind === 'genesis' || ev.$kind === 'genesis_v2') {
        const thread: ThreadObject = {
          root: { id: fromHex(target.id), entity: { feed: { id: fromHex(target.entity.feed.id), counter: String(counter) }, version: target.entity.version }, genesis: false },
          projection: emptyThreadProjection(target.entity.version),
        }
        pendingThreads.set(target.id, applyThreadGenesis(thread, ev))
        continue
      }
      data = { ...data, threads: data.threads.map((t) => (toHex(t.root.id) === target.id ? applyThread(t, ev, target.entity.version) : t)) }
      continue
    }

    if (env.kind === 'post') {
      if (ev.$kind === 'genesis' || ev.$kind === 'genesis_v2') {
        const threadHex = typeof ev.payload.thread === 'string' ? ev.payload.thread : ''
        const pending = threadHex ? pendingThreads.get(threadHex) : undefined
        if (pending) {
          const post: PostObject = {
            root: { id: fromHex(target.id), entity: { feed: { id: fromHex(target.entity.feed.id), counter: String(counter) }, version: target.entity.version }, genesis: false },
            projection: emptyPostProjection(target.entity.version),
          }
          const op = applyPost(post, ev, target.entity.version)
          const opPosts = new Map(data.opPosts)
          opPosts.set(threadHex, op)
          const last3 = new Map(data.last3)
          if (!last3.has(threadHex)) last3.set(threadHex, [])
          const variant = Object.keys(pending.projection)[0] as keyof ThreadProjectionData
          const dd = pending.projection[variant] as Record<string, unknown>
          const linked = { ...pending, projection: { [variant]: { ...dd, op: fromHex(target.id) } } as unknown as ThreadProjectionData }
          const pinned = new Set(new BoardProjection(data.board.projection).pinned().map(toHex))
          const head = data.threads.filter((t) => pinned.has(toHex(t.root.id)))
          const rest = data.threads.filter((t) => !pinned.has(toHex(t.root.id)))
          const merged = [...head, linked, ...rest]
          const threads = data.threads.length < BOARD_PAGE_THREADS ? merged.slice(0, BOARD_PAGE_THREADS) : merged
          data = { ...data, threads, opPosts, last3, decryptCtx: addPostsToDecryptContext(data.decryptCtx, [op]) }
          continue
        }
        if (threadHex && data.threads.some((t) => toHex(t.root.id) === threadHex)) {
          const post: PostObject = {
            root: { id: fromHex(target.id), entity: { feed: { id: fromHex(target.entity.feed.id), counter: String(counter) }, version: target.entity.version }, genesis: false },
            projection: emptyPostProjection(target.entity.version),
          }
          const appliedPost = applyPost(post, ev, target.entity.version)
          const last3 = new Map(data.last3)
          const list = last3.get(threadHex) ?? []
          if (!list.some((p) => toHex(p.root.id) === target.id)) {
            last3.set(threadHex, [...list, appliedPost].slice(-3))
            data = { ...data, last3, decryptCtx: addPostsToDecryptContext(data.decryptCtx, [appliedPost]) }
          }
          continue
        }
      }
      const opPosts = new Map(data.opPosts)
      for (const [k, v] of opPosts) if (toHex(v.root.id) === target.id) opPosts.set(k, applyPost(v, ev, target.entity.version))
      const last3 = new Map(data.last3)
      for (const [k, list] of last3) last3.set(k, list.map((p) => (toHex(p.root.id) === target.id ? applyPost(p, ev, target.entity.version) : p)))
      data = { ...data, opPosts, last3 }
    }
  }
  return { data }
}
