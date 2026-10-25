import { PostProjection, ThreadProjection } from '../core/bcs/types'
import { getForumModerators } from '../core/api/cache'
import { detectRoles, matchesAuthor, type RoleKind, type RoleOption } from '../core/intent/roles'
import { fromHex, toHex } from '../core/intent/crypto'
import { getSecretKeyBytes } from '../core/keys/storage'
import type { BoardViewData, ThreadViewData } from '../core/load'

export function detectViewRoles(data: ThreadViewData | BoardViewData): RoleOption[] {
  const master = getSecretKeyBytes()
  const forumIdBytes = fromHex(data.forumId)
  const base = {
    master,
    forumIdBytes,
    forumIdHex: data.forumId,
    boardUidHex: data.boardUid,
    moderators: data.moderators,
  }
  if ('threadUid' in data) {
    const opUid = toHex(new ThreadProjection(data.thread.projection).op())
    const opPost = data.posts.find((post) => toHex(post.root.id) === opUid)
    const pp = opPost ? new PostProjection(opPost.projection) : null
    const op =
      opPost && pp && matchesAuthor(master, forumIdBytes, pp.sender().pk, pp.sender().tweak)
        ? { tweak: pp.sender().tweak, pk: pp.sender().pk }
        : null
    return detectRoles({ ...base, threadUidHex: data.threadUid, op })
  }
  return detectRoles(base)
}

export const MOD_KINDS: RoleKind[] = ['forum_admin', 'forum_mod', 'board_mod', 'thread_admin', 'thread_mod']

const MOD_LEVELS: Record<string, number> = { forum_admin: 5, forum_mod: 4, board_mod: 3, thread_admin: 2, thread_mod: 1 }

export function hasModRole(kinds: RoleKind[]): boolean {
  return kinds.some((kind) => MOD_KINDS.includes(kind))
}

export function accessLevel(kinds: RoleKind[]): number {
  return kinds.reduce((max, kind) => Math.max(max, MOD_LEVELS[kind] ?? 0), 0)
}

export function detectForumRoles(relayUrl: string, forumId: string): RoleOption[] {
  const moderators = getForumModerators(relayUrl)
  if (!moderators) return []
  const forumIdHex = forumId.replace(/^0x/, '')
  return detectRoles({ master: getSecretKeyBytes(), forumIdBytes: fromHex(forumIdHex), forumIdHex, moderators })
}

const MOD_PREVIEW_KEY = 'ech-board-mod-preview'

export function modPreviewEnabled(): boolean {
  try {
    return localStorage.getItem(MOD_PREVIEW_KEY) === '1'
  } catch {
    return false
  }
}
