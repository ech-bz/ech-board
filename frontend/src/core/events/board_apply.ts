import { defineEvent, registerEvents, type DecodedEvent, type EventDesc } from './core'
import { f } from '../bcs/fields'
import { mapBoard, upgradeBoard } from './projection'
import { toHex, fromHex } from '../intent/crypto'
import type { BoardObject, ModeratorsData } from '../bcs/types'

export const genesis = defineEvent('board', 'genesis', [f.string('slug')])
export const upgrade = defineEvent('board', 'upgrade', [])
export const add_moderator = defineEvent('board', 'add_moderator', [f.address('moderator')])
export const del_moderator = defineEvent('board', 'del_moderator', [f.address('moderator')])
export const set_max_media = defineEvent('board', 'set_max_media', [f.u64('max_media')])
export const set_bump_limit = defineEvent('board', 'set_bump_limit', [f.u64('bump_limit')])
export const set_closed = defineEvent('board', 'set_closed', [f.bool('closed')])
export const set_deleted = defineEvent('board', 'set_deleted', [f.bool('deleted')])
export const new_thread = defineEvent('board', 'new_thread', [
  f.optU256('topic_hash'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.vecU256('vote_keys'), f.optU256('name_hash'),
])
export const new_thread_v2 = defineEvent('board', 'new_thread_v2', [
  f.optU256('topic_hash'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.optU256('name_hash'), f.vecU256('vote_keys'), f.bool('multi_vote'),
])
export const new_thread_v3 = defineEvent('board', 'new_thread_v3', [
  f.optU256('topic_hash'), f.bool('self_admin'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.optU256('name_hash'), f.vecU256('vote_keys'), f.bool('multi_vote'),
])
export const new_thread_migrate_v2 = defineEvent('board', 'new_thread_migrate_v2', [
  f.u64('timestamp_ms'), f.optU256('topic_hash'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.optU256('name_hash'), f.vecU256('vote_keys'), f.bool('multi_vote'),
])
export const set_description = defineEvent('board', 'set_description', [f.optU256('desc_hash')])
export const set_ignore_forum_bans = defineEvent('board', 'set_ignore_forum_bans', [f.bool('ignore')])
export const set_reactions = defineEvent('board', 'set_reactions', [f.vecU256('reactions')])
export const set_pinned = defineEvent('board', 'set_pinned', [f.vecAddress('pinned')])
export const new_post = defineEvent('board', 'new_post', [
  f.address('thread'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.vecU256('vote_keys'), f.optU256('name_hash'),
])
export const new_post_v2 = defineEvent('board', 'new_post_v2', [
  f.address('thread'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.optU256('name_hash'), f.vecU256('vote_keys'), f.bool('multi_vote'),
])
export const new_post_migrate_v2 = defineEvent('board', 'new_post_migrate_v2', [
  f.u64('timestamp_ms'), f.address('thread'), f.optU256('text_hash'), f.vecU256('media_hashes'), f.optU256('name_hash'), f.vecU256('vote_keys'), f.bool('multi_vote'),
])
export const ban = defineEvent('board', 'ban', [
  f.address('level'), f.u8('mask'), f.u256('ip_hash'), f.u256('reason_hash'), f.u64('expires'),
])
export const unban = defineEvent('board', 'unban', [f.address('level'), f.u8('mask'), f.u256('ip_hash')])

registerEvents('board', [
  genesis, upgrade, add_moderator, del_moderator, set_max_media, set_bump_limit, set_closed,
  set_deleted, new_thread, new_thread_v2, new_thread_v3, new_thread_migrate_v2, set_description,
  set_ignore_forum_bans, set_reactions, set_pinned, new_post, new_post_v2, new_post_migrate_v2,
  ban, unban,
])

export const BoardEvent = {
  upgrade: (): EventDesc => upgrade.encode(),
  addModerator: (addr: Uint8Array): EventDesc => add_moderator.encode(addr),
  delModerator: (addr: Uint8Array): EventDesc => del_moderator.encode(addr),
  setMaxMedia: (v: number): EventDesc => set_max_media.encode(v),
  setBumpLimit: (v: number): EventDesc => set_bump_limit.encode(v),
  setClosed: (v: boolean): EventDesc => set_closed.encode(v),
  setDeleted: (v: boolean): EventDesc => set_deleted.encode(v),
  newThread: (topicHash: Uint8Array | null, textHash: Uint8Array | null, mediaHashes: Uint8Array[], voteKeys: Uint8Array[], nameHash: Uint8Array | null): EventDesc =>
    new_thread.encode(topicHash, textHash, mediaHashes, voteKeys, nameHash),
  newThreadV2: (topicHash: Uint8Array | null, textHash: Uint8Array | null, mediaHashes: Uint8Array[], nameHash: Uint8Array | null, voteKeys: Uint8Array[], multiVote: boolean): EventDesc =>
    new_thread_v2.encode(topicHash, textHash, mediaHashes, nameHash, voteKeys, multiVote),
  newThreadV3: (topicHash: Uint8Array | null, selfAdmin: boolean, textHash: Uint8Array | null, mediaHashes: Uint8Array[], nameHash: Uint8Array | null, voteKeys: Uint8Array[], multiVote: boolean): EventDesc =>
    new_thread_v3.encode(topicHash, selfAdmin, textHash, mediaHashes, nameHash, voteKeys, multiVote),
  newPost: (threadId: Uint8Array, textHash: Uint8Array | null, mediaHashes: Uint8Array[], voteKeys: Uint8Array[], nameHash: Uint8Array | null): EventDesc =>
    new_post.encode(threadId, textHash, mediaHashes, voteKeys, nameHash),
  newPostV2: (threadId: Uint8Array, textHash: Uint8Array | null, mediaHashes: Uint8Array[], nameHash: Uint8Array | null, voteKeys: Uint8Array[], multiVote: boolean): EventDesc =>
    new_post_v2.encode(threadId, textHash, mediaHashes, nameHash, voteKeys, multiVote),
  setDescription: (hash: Uint8Array | null): EventDesc => set_description.encode(hash),
  setIgnoreForumBans: (v: boolean): EventDesc => set_ignore_forum_bans.encode(v),
  setReactions: (reactions: Uint8Array[]): EventDesc => set_reactions.encode(reactions),
  setPinned: (pinned: Uint8Array[]): EventDesc => set_pinned.encode(pinned),
  banUser: (level: Uint8Array, mask: number, ipHash: Uint8Array, reasonHash: Uint8Array, expires: number): EventDesc =>
    ban.encode(level, mask, ipHash, reasonHash, expires),
  unbanUser: (level: Uint8Array, mask: number, ipHash: Uint8Array): EventDesc =>
    unban.encode(level, mask, ipHash),
}

export function applyBoard(board: BoardObject, ev: DecodedEvent, version: number): BoardObject {
  switch (ev.$kind) {
    case 'upgrade':
      return { ...board, root: { ...board.root, entity: { ...board.root.entity, version } }, projection: upgradeBoard(board.projection, version) }
    case 'set_description':
      return mapBoard(board, d => ({ ...d, description_hash: ev.payload.desc_hash ? fromHex(ev.payload.desc_hash) : null }))
    case 'set_max_media':
      return mapBoard(board, d => ({ ...d, max_media: String(ev.payload.max_media) }))
    case 'set_bump_limit':
      return mapBoard(board, d => ({ ...d, bump_limit: String(ev.payload.bump_limit) }))
    case 'set_closed':
      return mapBoard(board, d => ({ ...d, closed: ev.payload.closed }))
    case 'set_deleted':
      return mapBoard(board, d => ({ ...d, deleted: ev.payload.deleted }))
    case 'set_ignore_forum_bans':
      return mapBoard(board, d => ({ ...d, ignore_forum_bans: ev.payload.ignore }))
    case 'set_reactions':
      return mapBoard(board, d => ({ ...d, reactions: (ev.payload.reactions ?? []).map((h: string) => fromHex(h)) }))
    case 'set_pinned':
      return mapBoard(board, d => ({ ...d, pinned: (ev.payload.pinned ?? []).map((h: string) => fromHex(h)) }))
    default:
      return board
  }
}

export function applyBoardModerators(moderators: ModeratorsData | null | undefined, ev: DecodedEvent): ModeratorsData | null | undefined {
  if (!moderators) return moderators
  const mod = ev.payload.moderator
  if (ev.$kind === 'add_moderator') {
    if (moderators.board_mods.some((a: Uint8Array) => toHex(a) === mod)) return moderators
    return { ...moderators, board_mods: [...moderators.board_mods, fromHex(mod)] }
  }
  if (ev.$kind === 'del_moderator') {
    return { ...moderators, board_mods: moderators.board_mods.filter((a: Uint8Array) => toHex(a) !== mod) }
  }
  return moderators
}
