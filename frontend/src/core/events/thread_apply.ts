import { defineEvent, registerEvents, type DecodedEvent, type EventDesc } from './core'
import { f } from '../bcs/fields'
import { mapThread, mapPost, upgradeThread } from './projection'
import { toHex, fromHex } from '../intent/crypto'
import type { ThreadObject, PostObject, ThreadProjectionData, PostProjectionData, ModeratorsData } from '../bcs/types'
import type { PostProjectionVariant } from './projection'

export const genesis = defineEvent('thread', 'genesis', [
  f.address('board'),
  f.u64('number'),
  f.optU256('topic_hash'),
])

export const genesis_v2 = defineEvent('thread', 'genesis_v2', [
  f.address('board'),
  f.u64('number'),
  f.optU256('topic_hash'),
  f.optAddress('admin'),
])

export const upgrade = defineEvent('thread', 'upgrade', [])
export const add_moderator = defineEvent('thread', 'add_moderator', [f.address('moderator')])
export const del_moderator = defineEvent('thread', 'del_moderator', [f.address('moderator')])
export const set_closed = defineEvent('thread', 'set_closed', [f.bool('closed')])
export const set_deleted = defineEvent('thread', 'set_deleted', [f.bool('deleted')])
export const set_topic = defineEvent('thread', 'set_topic', [f.optU256('topic_hash')])
export const set_admin = defineEvent('thread', 'set_admin', [f.optAddress('admin')])
export const new_post = defineEvent('thread', 'new_post', [f.address('post')])
export const ban = defineEvent('thread', 'ban', [
  f.address('level'), f.u8('mask'), f.u256('ip_hash'), f.u256('reason_hash'), f.u64('expires'),
])
export const unban = defineEvent('thread', 'unban', [f.address('level'), f.u8('mask'), f.u256('ip_hash')])
export const post_set_deleted = defineEvent('thread', 'post_set_deleted', [f.bool('deleted')])
export const post_set_text = defineEvent('thread', 'post_set_text', [f.optU256('hash')])

registerEvents('thread', [
  genesis, genesis_v2, upgrade, add_moderator, del_moderator, set_closed, set_deleted, set_topic,
  set_admin, new_post, ban, unban, post_set_deleted, post_set_text,
])

export const ThreadEvent = {
  upgrade: (): EventDesc => upgrade.encode(),
  addModerator: (addr: Uint8Array): EventDesc => add_moderator.encode(addr),
  delModerator: (addr: Uint8Array): EventDesc => del_moderator.encode(addr),
  setClosed: (v: boolean): EventDesc => set_closed.encode(v),
  setDeleted: (v: boolean): EventDesc => set_deleted.encode(v),
  postSetDeleted: (v: boolean): EventDesc => post_set_deleted.encode(v),
  postSetText: (hash: Uint8Array | null): EventDesc => post_set_text.encode(hash),
  setAdmin: (addr: Uint8Array | null): EventDesc => set_admin.encode(addr),
  setTopic: (hash: Uint8Array | null): EventDesc => set_topic.encode(hash),
  banUser: (level: Uint8Array, mask: number, ipHash: Uint8Array, reasonHash: Uint8Array, expires: number): EventDesc =>
    ban.encode(level, mask, ipHash, reasonHash, expires),
  unbanUser: (level: Uint8Array, mask: number, ipHash: Uint8Array): EventDesc =>
    unban.encode(level, mask, ipHash),
}

export function applyThread(thread: ThreadObject, ev: DecodedEvent, version: number): ThreadObject {
  switch (ev.$kind) {
    case 'new_post':
      return mapThread(thread, d => ({
        ...d,
        posts: { ...d.posts, counter: String(Number(d.posts.counter) + 1) },
      }))
    case 'set_closed':
      return mapThread(thread, d => ({ ...d, closed: ev.payload.closed }))
    case 'set_deleted':
      return mapThread(thread, d => ({ ...d, deleted: ev.payload.deleted }))
    case 'set_topic':
      return mapThread(thread, d => ({ ...d, topic_hash: ev.payload.topic_hash ? fromHex(ev.payload.topic_hash) : null }))
    case 'set_admin':
      return mapThread(thread, d => ({ ...d, admin: ev.payload.admin ? fromHex(ev.payload.admin) : null }))
    case 'upgrade':
      return { ...thread, root: { ...thread.root, entity: { ...thread.root.entity, version } }, projection: upgradeThread(thread.projection, version) }
    default:
      return thread
  }
}

export function applyPostFlags(post: PostObject, ev: DecodedEvent): PostObject {
  switch (ev.$kind) {
    case 'ban':
    case 'unban': {
      const ban = ev.$kind === 'ban' ? ev.payload : null
      return mapPost(post, d => ({
        ...d,
        banned: ban ? { level: fromHex(ban.level), mask: ban.mask, ip_hash: fromHex(ban.ip_hash) } : null,
      }))
    }
    case 'post_set_deleted':
      return mapPost(post, d => ({ ...d, deleted: ev.payload.deleted }))
    case 'post_set_text': {
      const hash = ev.payload.hash
      const d = post.projection[Object.keys(post.projection)[0] as keyof PostProjectionData] as PostProjectionVariant
      let np = mapPost(post, dd => ({ ...dd, text_hash: hash ? fromHex(hash) : null }))
      if (willPostBecomeEmpty(d, hash)) np = mapPost(np, dd => ({ ...dd, deleted: true }))
      return np
    }
    default:
      return post
  }
}

function willPostBecomeEmpty(d: PostProjectionVariant, hash: string | null | undefined): boolean {
  return !d.deleted && !hash && d.media_hashes.length === 0
}

export function postsDeletedDelta(post: PostObject, ev: DecodedEvent): number {
  if (ev.$kind === 'post_set_deleted') return ev.payload.deleted ? 1 : -1
  if (ev.$kind === 'post_set_text') {
    const hash = ev.payload.hash
    const d = post.projection[Object.keys(post.projection)[0] as keyof PostProjectionData] as PostProjectionVariant
    return willPostBecomeEmpty(d, hash) ? 1 : 0
  }
  return 0
}

export function adjustPostsDeleted(thread: ThreadObject, delta: number): ThreadObject {
  return mapThread(thread, d => ({ ...d, posts_deleted: String(Number(('posts_deleted' in d ? d.posts_deleted : '0') ?? 0) + delta) }))
}

export function applyThreadModerators(moderators: ModeratorsData | null | undefined, ev: DecodedEvent): ModeratorsData | null | undefined {
  if (!moderators) return moderators
  const mod = ev.payload.moderator
  if (ev.$kind === 'add_moderator') {
    if (moderators.thread_mods.some((a: Uint8Array) => toHex(a) === mod)) return moderators
    return { ...moderators, thread_mods: [...moderators.thread_mods, fromHex(mod)] }
  }
  if (ev.$kind === 'del_moderator') {
    return { ...moderators, thread_mods: moderators.thread_mods.filter((a: Uint8Array) => toHex(a) !== mod) }
  }
  return moderators
}

function emptyRegistry(): { counter: string; entries: { id: Uint8Array; size: string }; identities: { id: Uint8Array; size: string }; index: { id: Uint8Array; size: string } } {
  const table = () => ({ id: new Uint8Array(32), size: '0' })
  return { counter: '0', entries: table(), identities: table(), index: table() }
}

export function emptyThreadProjection(version: number): ThreadProjectionData {
  const base = {
    board: new Uint8Array(32),
    number: '0',
    topic_hash: null,
    op: new Uint8Array(32),
    closed: false,
    deleted: false,
    admin: null,
    mods: { id: new Uint8Array(32), size: '0' },
    bans: { level: new Uint8Array(32), ip32: emptyRegistry(), ip24: emptyRegistry(), ip20: emptyRegistry(), ip16: emptyRegistry() },
    posts: { id: new Uint8Array(32), counter: '0' },
    last_3: [],
  }
  if (version === 1) return { V1: { ...base, pinned: false } } as unknown as ThreadProjectionData
  if (version === 2) return { V2: base } as unknown as ThreadProjectionData
  return { V3: { ...base, posts_deleted: '0' } } as unknown as ThreadProjectionData
}

export function applyThreadGenesis(thread: ThreadObject, ev: DecodedEvent): ThreadObject {
  const variant = Object.keys(thread.projection)[0] as keyof ThreadProjectionData
  const d = thread.projection[variant] as Record<string, unknown>
  return {
    ...thread,
    projection: {
      [variant]: {
        ...d,
        board: fromHex(ev.payload.board),
        number: String(ev.payload.number),
        topic_hash: ev.payload.topic_hash ? fromHex(ev.payload.topic_hash) : null,
        admin: ev.payload.admin ? fromHex(ev.payload.admin) : null,
      },
    } as unknown as ThreadProjectionData,
  }
}
