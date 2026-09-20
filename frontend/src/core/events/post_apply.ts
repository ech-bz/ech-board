import { defineEvent, registerEvents, type DecodedEvent, type EventDesc } from './core'
import { f } from '../bcs/fields'
import { mapPost, mapProjection, upgradePost, clonePairs } from './projection'
import { toHex, fromHex } from '../intent/crypto'
import type { PostObject, PostProjectionData } from '../bcs/types'

export const genesis = defineEvent('post', 'genesis', [
  f.address('thread'),
  f.u64('number'),
  f.u64('timestamp_ms'),
  f.optU256('name_hash'),
  f.optU256('text_hash'),
  f.vecU256('media_hashes'),
  f.vecU256('vote_keys'),
])

export const genesis_v2 = defineEvent('post', 'genesis_v2', [
  f.address('thread'),
  f.u64('number'),
  f.u64('timestamp_ms'),
  f.optU256('name_hash'),
  f.optU256('text_hash'),
  f.vecU256('media_hashes'),
  f.vecU256('vote_keys'),
  f.bool('multi_vote'),
])

export const upgrade = defineEvent('post', 'upgrade', [])

export const set_deleted = defineEvent('post', 'set_deleted', [f.bool('deleted')])
export const set_text = defineEvent('post', 'set_text', [f.optU256('text_hash')])
export const ban_media = defineEvent('post', 'ban_media', [f.vecU256('media_hashes')])
export const unban_media = defineEvent('post', 'unban_media', [f.vecU256('media_hashes')])
export const remove_media = defineEvent('post', 'remove_media', [f.vecU256('media_hashes')])
export const set_reaction = defineEvent('post', 'set_reaction', [f.u256('reaction_hash')])
export const set_reaction_v2 = defineEvent('post', 'set_reaction_v2', [f.optU256('old'), f.u256('new')])
export const vote = defineEvent('post', 'vote', [f.u256('option_hash')])
export const vote_v2 = defineEvent('post', 'vote_v2', [f.vecU256('options')])
export const set_banned = defineEvent('post', 'set_banned', [f.optBanKey('ban')])
export const set_mod_note = defineEvent('post', 'set_mod_note', [f.optU256('mod_note')])

registerEvents('post', [
  genesis, genesis_v2, upgrade, set_deleted, set_text, ban_media, unban_media,
  remove_media, set_reaction, set_reaction_v2, vote, vote_v2, set_banned, set_mod_note,
])

export const PostEvent = {
  upgrade: (): EventDesc => upgrade.encode(),
  removeMedia: (hashes: Uint8Array[]): EventDesc => remove_media.encode(hashes),
  banMedia: (hashes: Uint8Array[]): EventDesc => ban_media.encode(hashes),
  unbanMedia: (hashes: Uint8Array[]): EventDesc => unban_media.encode(hashes),
  setDeleted: (v: boolean): EventDesc => set_deleted.encode(v),
  setReaction: (reactionHash: Uint8Array): EventDesc => set_reaction.encode(reactionHash),
  setReactionV2: (old: Uint8Array | null, reactionHash: Uint8Array): EventDesc => set_reaction_v2.encode(old, reactionHash),
  vote: (optionHash: Uint8Array): EventDesc => vote.encode(optionHash),
  voteV2: (options: Uint8Array[]): EventDesc => vote_v2.encode(options),
}

export function emptyPostProjection(version: number): PostProjectionData {
  const base = {
    sender: { pk: new Uint8Array(32), tweak: new Uint8Array(32) },
    thread: new Uint8Array(32),
    number: '0',
    uid: new Uint8Array(0),
    timestamp_ms: '0',
    deleted: false,
    banned: null,
    text_hash: null,
    media_hashes: [],
    reactions: [],
    votes: [],
    name_hash: null,
    trip: null,
    geo: null,
    mod_note: null,
  }
  if (version === 1) return { V1: base } as unknown as PostProjectionData
  if (version === 2) return { V2: { ...base, multi_vote: false } } as unknown as PostProjectionData
  return { V3: { ...base, banned_media: [], multi_vote: false } } as unknown as PostProjectionData
}

export function applyPostGenesis(post: PostObject, ev: DecodedEvent): PostObject {
  const variant = Object.keys(post.projection)[0] as keyof PostProjectionData
  const d = post.projection[variant] as Record<string, unknown>
  const votes: [Uint8Array, string][] = (ev.payload.vote_keys ?? []).map((k: string) => [fromHex(k), '0'] as [Uint8Array, string])
  const projection = {
    ...d,
    sender: { pk: fromHex(ev.sender.pk), tweak: fromHex(ev.sender.tweak) },
    thread: fromHex(ev.payload.thread),
    number: String(ev.payload.number),
    uid: ev.responses.uid ? fromHex(ev.responses.uid) : new Uint8Array(0),
    timestamp_ms: String(ev.payload.timestamp_ms),
    name_hash: ev.payload.name_hash ? fromHex(ev.payload.name_hash) : null,
    text_hash: ev.payload.text_hash ? fromHex(ev.payload.text_hash) : null,
    media_hashes: (ev.payload.media_hashes ?? []).map((h: string) => fromHex(h)),
    votes,
    multi_vote: ev.payload.multi_vote,
    trip: ev.responses.tripcode ? { secured: ev.responses.tripcode.secured, trip: ev.responses.tripcode.trip } : null,
    geo: ev.responses.geo ?? null,
  }
  return { ...post, projection: { [variant]: projection } as unknown as PostProjectionData }
}

export function applyReaction(post: PostObject, old: string | null, next: string): PostObject {
  return {
    ...post,
    projection: mapProjection(post.projection, (d) => {
      const reactions = clonePairs(d.reactions)
      const dec = (hash: string) => {
        const i = reactions.findIndex(([h]) => toHex(h) === hash)
        if (i < 0) return
        const cnt = Number(reactions[i][1]) - 1
        if (cnt <= 0) reactions.splice(i, 1)
        else reactions[i][1] = String(cnt)
      }
      const inc = (hash: string) => {
        const i = reactions.findIndex(([h]) => toHex(h) === hash)
        if (i >= 0) reactions[i][1] = String(Number(reactions[i][1]) + 1)
        else reactions.push([fromHex(hash), '1'])
      }
      if (old) dec(old)
      if (next && next !== old) inc(next)
      return { ...d, reactions }
    }),
  }
}

export function applyVote(post: PostObject, options: string[]): PostObject {
  return {
    ...post,
    projection: mapProjection(post.projection, (d) => {
      const votes = clonePairs(d.votes)
      const seen = new Set<string>()
      for (const opt of options) {
        if (seen.has(opt)) continue
        seen.add(opt)
        const i = votes.findIndex(([h]) => toHex(h) === opt)
        if (i >= 0) votes[i][1] = String(Number(votes[i][1]) + 1)
        else votes.push([fromHex(opt), '1'])
      }
      return { ...d, votes }
    }),
  }
}

export function applyPost(post: PostObject, ev: DecodedEvent, version: number): PostObject {
  switch (ev.$kind) {
    case 'genesis':
    case 'genesis_v2':
      return applyPostGenesis(post, ev)
    case 'upgrade':
      return { ...post, root: { ...post.root, entity: { ...post.root.entity, version } }, projection: upgradePost(post.projection, version) }
    case 'set_deleted':
      return mapPost(post, d => ({ ...d, deleted: ev.payload.deleted }))
    case 'set_text':
      return mapPost(post, d => ({ ...d, text_hash: ev.payload.text_hash ? fromHex(ev.payload.text_hash) : null }))
    case 'ban_media': {
      const add = new Set<string>(ev.payload.media_hashes ?? [])
      return mapPost(post, d => {
        const existing = 'banned_media' in d ? d.banned_media : []
        const set = new Set<string>(existing.map((h: Uint8Array) => toHex(h)))
        for (const h of add) set.add(h)
        return { ...d, banned_media: [...set].map(h => fromHex(h)) }
      })
    }
    case 'unban_media': {
      const remove = new Set<string>(ev.payload.media_hashes ?? [])
      return mapPost(post, d => {
        const existing = 'banned_media' in d ? d.banned_media : []
        const set = new Set<string>(existing.map((h: Uint8Array) => toHex(h)))
        for (const h of remove) set.delete(h)
        return { ...d, banned_media: [...set].map(h => fromHex(h)) }
      })
    }
    case 'set_reaction_v2':
      return applyReaction(post, ev.payload.old, ev.payload.new)
    case 'vote_v2':
      return applyVote(post, ev.payload.options)
    case 'set_banned': {
      const ban = ev.payload.ban
      const banned = ban
        ? { level: fromHex(ban.level), mask: ban.mask, ip_hash: fromHex(ban.ip_hash) }
        : null
      return mapPost(post, d => ({ ...d, banned }))
    }
    case 'set_mod_note':
      return mapPost(post, d => ({ ...d, mod_note: ev.payload.mod_note ? fromHex(ev.payload.mod_note) : null }))
    default:
      return post
  }
}
