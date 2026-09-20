import { PostProjection, type PostObject, type PostPartInput, type MediaMetaData, type ModeratorsData } from '../core/bcs/types'
import type { DecryptContext } from '../core/intent/decrypt'
import { canPostAction, canUnbanMedia, SELF_MOD_WINDOW_MS, type PostAction } from '../core/intent/capabilities'
import { matchesAuthor, roleForAddress, type PostRole, type RoleKind } from '../core/intent/roles'
import { postMedia, postReactions, reactionUrl, type MediaInfo, type ReactionInfo } from '../core/media'
import { partsToPlainText, postMeta, postParts, type PostRef } from '../core/posts'
import { senderAddress, shardIndex, toHex } from '../core/intent/crypto'

export interface PostRecord {
  readonly uid: string
  readonly threadUid: string
  readonly number: number
  readonly relNum: number
  readonly op: boolean
  readonly post: PostObject
  readonly senderPk: Uint8Array
  readonly senderTweak: Uint8Array
  readonly name: string | null
  readonly tripText: string | null
  readonly tripSecured: boolean
  readonly flagUrl: string | null
  readonly timestampMs: number
  readonly hasText: boolean
  readonly parts: PostPartInput[] | null
  readonly plain: string
  readonly lineCount: number
  readonly refs: readonly PostRef[]
  readonly media: readonly MediaInfo[]
  readonly reactions: readonly ReactionInfo[]
  readonly reactionGlyph: string | null
  readonly myReaction: string | null
  readonly myReactionBase: string | null
  readonly repliedMine: boolean
  readonly deleted: boolean
  readonly banned: boolean
  readonly bannedLevel: string | null
  readonly mine: boolean
  readonly role: PostRole
  readonly canEdit: boolean
  readonly canDelete: boolean
  readonly canRestore: boolean
  readonly canBanMedia: boolean
  readonly canUnbanMedia: boolean
  readonly deadline: number | null
}

export interface RecordInput {
  post: PostObject
  threadUid: string
  relayUrl: string
  forumIdBytes: Uint8Array
  boardUid: string
  contentMap: Map<string, Uint8Array>
  mediaMeta: Map<string, MediaMetaData>
  decryptCtx: DecryptContext
  moderators: ModeratorsData
  master: Uint8Array
  boardReactions: Uint8Array[]
  roleKinds: RoleKind[]
  op: boolean
  opDeleted: boolean
  relNum: number
  refs: readonly PostRef[]
  myReaction: string | null
  myReactionBase: string | null
  now: number
}

function walkRepliedMine(parts: PostPartInput[], decryptCtx: DecryptContext): boolean {
  const { postAuthors, myAuthorTweaks } = decryptCtx
  for (const p of parts) {
    if ('ReplyTo' in p) {
      const pk = postAuthors.get(toHex(p.ReplyTo.post))
      if (pk && myAuthorTweaks.has(toHex(pk))) return true
    } else if ('Bold' in p && walkRepliedMine(p.Bold, decryptCtx)) return true
    else if ('Italic' in p && walkRepliedMine(p.Italic, decryptCtx)) return true
    else if ('Code' in p && walkRepliedMine(p.Code, decryptCtx)) return true
    else if ('Underline' in p && walkRepliedMine(p.Underline, decryptCtx)) return true
    else if ('Overline' in p && walkRepliedMine(p.Overline, decryptCtx)) return true
    else if ('Spoiler' in p && walkRepliedMine(p.Spoiler, decryptCtx)) return true
    else if ('Strike' in p && walkRepliedMine(p.Strike, decryptCtx)) return true
    else if ('Sup' in p && walkRepliedMine(p.Sup, decryptCtx)) return true
    else if ('Sub' in p && walkRepliedMine(p.Sub, decryptCtx)) return true
    else if ('Link' in p && walkRepliedMine(p.Link.children, decryptCtx)) return true
  }
  return false
}

function glyphOf(boardReactions: Uint8Array[], postId: Uint8Array, relayUrl: string, boardUid: string): string | null {
  if (boardReactions.length === 0) return null
  return reactionUrl(relayUrl, boardUid, toHex(boardReactions[shardIndex(postId, boardReactions.length)]))
}

function reactionBubbles(post: PostObject, relayUrl: string, boardUid: string, myReaction: string | null, base: string | null): ReactionInfo[] {
  const seen = new Map<string, number>()
  for (const item of postReactions(post, relayUrl, boardUid)) {
    let count = item.count
    if (item.hex === base) {
      if (myReaction !== item.hex) count -= 1
    } else if (myReaction === item.hex) {
      count += 1
    }
    if (count > 0) seen.set(item.hex, count)
  }
  if (myReaction && !seen.has(myReaction)) seen.set(myReaction, 1)
  return [...seen.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([hex, count]) => ({ url: reactionUrl(relayUrl, boardUid, hex), count, hex }))
}

export function buildPostRecord(input: RecordInput): PostRecord {
  const { post, contentMap } = input
  const pp = new PostProjection(post.projection)
  const meta = postMeta(post)
  const sender = pp.sender()
  const nameHash = pp.name_hash()
  const nameBytes = nameHash ? contentMap.get(toHex(nameHash)) : undefined
  const trip = pp.trip()
  const geo = pp.geo()
  const textHash = meta.textHash
  const parts = textHash === null ? [] : contentMap.has(textHash) ? postParts(post, contentMap) : null
  const plain = parts ? partsToPlainText(parts) : ''
  const timestampMs = Number(pp.timestamp_ms())
  const deleted = meta.deleted || input.opDeleted
  const ageMs = input.now - timestampMs
  const mine = matchesAuthor(input.master, input.forumIdBytes, sender.pk, sender.tweak)
  const allowed = (action: PostAction) => canPostAction(action, input.roleKinds, mine, ageMs)
  return {
    uid: meta.uid,
    threadUid: input.threadUid,
    number: meta.number,
    relNum: input.relNum,
    op: input.op,
    post,
    senderPk: sender.pk,
    senderTweak: sender.tweak,
    name: nameBytes ? new TextDecoder().decode(nameBytes) || null : null,
    tripText: trip ? trip.trip : null,
    tripSecured: trip ? trip.secured : false,
    flagUrl: geo != null ? `/static/flags/${String(geo).padStart(3, '0')}.svg` : null,
    timestampMs,
    hasText: textHash !== null,
    parts,
    plain,
    lineCount: plain.split('\n').length,
    refs: input.refs,
    media: postMedia(post, input.relayUrl, input.boardUid, input.threadUid, input.mediaMeta, true),
    reactions: reactionBubbles(post, input.relayUrl, input.boardUid, input.myReaction, input.myReactionBase),
    reactionGlyph: glyphOf(input.boardReactions, post.root.id, input.relayUrl, input.boardUid),
    myReaction: input.myReaction,
    myReactionBase: input.myReactionBase,
    repliedMine: parts ? walkRepliedMine(parts, input.decryptCtx) : false,
    deleted,
    banned: pp.banned() !== null,
    bannedLevel: pp.banned() ? toHex(pp.banned()!.level) : null,
    mine,
    role: input.op ? 'op' : roleForAddress(senderAddress(sender.pk), input.moderators),
    canEdit: allowed('edit') && !deleted,
    canDelete: allowed('delete') && !deleted,
    canRestore: allowed('restore') && deleted,
    canBanMedia: allowed('ban_media') && !deleted && pp.media_hashes().length > 0,
    canUnbanMedia: canUnbanMedia(input.roleKinds) && !deleted && pp.banned_media().length > 0,
    deadline: mine && ageMs <= SELF_MOD_WINDOW_MS ? timestampMs + SELF_MOD_WINDOW_MS : null,
  }
}

export function missingBlobs(post: PostObject, contentMap: Map<string, Uint8Array>, mediaMeta: Map<string, MediaMetaData>): { text: string | null; name: string | null; media: string[] } {
  const pp = new PostProjection(post.projection)
  const meta = postMeta(post)
  const nameHash = pp.name_hash()
  const name = nameHash ? toHex(nameHash) : null
  const media: string[] = []
  for (const h of pp.media_hashes()) {
    const hex = toHex(h)
    if (!mediaMeta.has(hex)) media.push(hex)
  }
  return {
    text: meta.textHash !== null && !contentMap.has(meta.textHash) ? meta.textHash : null,
    name: name !== null && !contentMap.has(name) ? name : null,
    media,
  }
}
