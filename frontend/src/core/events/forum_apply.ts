import { defineEvent, registerEvents, type DecodedEvent, type EventDesc } from './core'
import { f } from '../bcs/fields'
import { mapForum } from './projection'
import { toHex, fromHex } from '../intent/crypto'
import type { ForumObject, ModeratorsData } from '../bcs/types'

export const genesis = defineEvent('forum', 'genesis', [f.address('admin')])
export const upgrade = defineEvent('forum', 'upgrade', [])
export const add_moderator = defineEvent('forum', 'add_moderator', [f.address('moderator')])
export const del_moderator = defineEvent('forum', 'del_moderator', [f.address('moderator')])
export const new_board = defineEvent('forum', 'new_board', [
  f.string('slug'), f.u64('max_media'), f.u64('bump_limit'), f.optU256('desc_hash'),
])
export const set_timestamp_precision = defineEvent('forum', 'set_timestamp_precision', [f.u64('precision')])
export const ban = defineEvent('forum', 'ban', [
  f.address('level'), f.u8('mask'), f.u256('ip_hash'), f.u256('reason_hash'), f.u64('expires'),
])
export const unban = defineEvent('forum', 'unban', [f.address('level'), f.u8('mask'), f.u256('ip_hash')])

registerEvents('forum', [
  genesis, upgrade, add_moderator, del_moderator, new_board, set_timestamp_precision, ban, unban,
])

export const ForumEvent = {
  upgrade: (): EventDesc => upgrade.encode(),
  addModerator: (addr: Uint8Array): EventDesc => add_moderator.encode(addr),
  delModerator: (addr: Uint8Array): EventDesc => del_moderator.encode(addr),
  newBoard: (slug: string, descriptionHash: Uint8Array | null, maxMedia: number, bumpLimit: number): EventDesc =>
    new_board.encode(slug, maxMedia, bumpLimit, descriptionHash),
  banUser: (level: Uint8Array, mask: number, ipHash: Uint8Array, reasonHash: Uint8Array, expires: number): EventDesc =>
    ban.encode(level, mask, ipHash, reasonHash, expires),
  unbanUser: (level: Uint8Array, mask: number, ipHash: Uint8Array): EventDesc =>
    unban.encode(level, mask, ipHash),
  setTimestampPrecision: (precision: number): EventDesc => set_timestamp_precision.encode(precision),
}

export function applyForum(forum: ForumObject, ev: DecodedEvent, version: number): ForumObject {
  switch (ev.$kind) {
    case 'upgrade':
      return { ...forum, root: { ...forum.root, entity: { ...forum.root.entity, version } } }
    case 'set_timestamp_precision':
      return mapForum(forum, d => ({ ...d, timestamp_precision_ms: String(ev.payload.precision) }))
    default:
      return forum
  }
}

export function applyForumModerators(moderators: ModeratorsData | null | undefined, ev: DecodedEvent): ModeratorsData | null | undefined {
  if (!moderators) return moderators
  const mod = ev.payload.moderator
  if (ev.$kind === 'add_moderator') {
    if (moderators.forum_mods.some((a: Uint8Array) => toHex(a) === mod)) return moderators
    return { ...moderators, forum_mods: [...moderators.forum_mods, fromHex(mod)] }
  }
  if (ev.$kind === 'del_moderator') {
    return { ...moderators, forum_mods: moderators.forum_mods.filter((a: Uint8Array) => toHex(a) !== mod) }
  }
  return moderators
}
