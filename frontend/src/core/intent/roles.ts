import { derivePostKeypair, senderAddress, computeTweak, toHex } from './crypto'
import type { MessageKey } from '../../ui/i18n/ru'

export type RoleKind =
  | 'anonymous'
  | 'op'
  | 'forum_mod'
  | 'board_mod'
  | 'thread_mod'
  | 'forum_admin'
  | 'thread_admin'

export interface RoleOption {
  kind: RoleKind
  labelKey: MessageKey
  addressHex?: string
  tweakArgs?: (Uint8Array | string)[]
  rawTweak?: Uint8Array
  selectable?: boolean
}

export function matchesAuthor(master: Uint8Array, forumId: Uint8Array, pk: Uint8Array, tweak: Uint8Array): boolean {
  const derived = derivePostKeypair(master, forumId, tweak)
  return toHex(derived.publicKey) === toHex(pk)
}

function roleCandidates(master: Uint8Array, forumId: Uint8Array, scopeIdHex: string, tag: 'moder' | 'admin'): { address: Uint8Array; tweakArgs: (Uint8Array | string)[] }[] {
  const idStr = scopeIdHex.replace(/^0x/, '')
  const tweak = computeTweak(idStr, tag)
  const { publicKey } = derivePostKeypair(master, forumId, tweak)
  return [{ address: senderAddress(publicKey), tweakArgs: [idStr, tag] }]
}

function inList(addr: Uint8Array, list: Uint8Array[]): boolean {
  const hex = toHex(addr)
  return list.some(a => toHex(a) === hex)
}

function pushRole(
  roles: RoleOption[],
  kind: RoleKind,
  labelKey: MessageKey,
  list: Uint8Array[],
  cands: { address: Uint8Array; tweakArgs: (Uint8Array | string)[] }[],
): void {
  for (const c of cands) {
    if (inList(c.address, list)) {
      roles.push({ kind, labelKey, addressHex: toHex(c.address), tweakArgs: c.tweakArgs })
      return
    }
  }
}

export interface Moderators {
  forum_admin: Uint8Array | null
  forum_mods: Uint8Array[]
  board_mods: Uint8Array[]
  thread_mods: Uint8Array[]
  thread_admin: Uint8Array | null
}

export interface RoleContext {
  master: Uint8Array
  forumIdBytes: Uint8Array
  forumIdHex: string
  boardUidHex?: string
  threadUidHex?: string
  moderators: Moderators
  op?: { tweak: Uint8Array; pk: Uint8Array } | null
}

export function detectRoles(ctx: RoleContext): RoleOption[] {
  const roles: RoleOption[] = [{ kind: 'anonymous', labelKey: 'roles.anonymous' }]
  const mods = ctx.moderators
  const adminList = (a: Uint8Array | null) => (a ? [a] : [])

  pushRole(roles, 'forum_mod', 'roles.forumMod', mods.forum_mods, roleCandidates(ctx.master, ctx.forumIdBytes, ctx.forumIdHex, 'moder'))

  if (ctx.boardUidHex) {
    pushRole(roles, 'board_mod', 'roles.boardMod', mods.board_mods, roleCandidates(ctx.master, ctx.forumIdBytes, ctx.boardUidHex, 'moder'))
  }

  if (ctx.threadUidHex) {
    pushRole(roles, 'thread_mod', 'roles.threadMod', mods.thread_mods, roleCandidates(ctx.master, ctx.forumIdBytes, ctx.threadUidHex, 'moder'))
    pushRole(roles, 'thread_admin', 'roles.threadAdmin', adminList(mods.thread_admin), roleCandidates(ctx.master, ctx.forumIdBytes, ctx.threadUidHex, 'admin'))
    if (!roles.some((role) => role.kind === 'thread_admin') && ctx.op && mods.thread_admin) {
      const opAddress = senderAddress(ctx.op.pk)
      if (toHex(opAddress) === toHex(mods.thread_admin)) {
        roles.push({
          kind: 'thread_admin',
          labelKey: 'roles.threadAdmin',
          addressHex: toHex(opAddress),
          rawTweak: ctx.op.tweak,
          selectable: false,
        })
      }
    }
  }

  pushRole(roles, 'forum_admin', 'roles.forumAdmin', adminList(mods.forum_admin), roleCandidates(ctx.master, ctx.forumIdBytes, ctx.forumIdHex, 'admin'))

  if (ctx.op) {
    roles.push({
      kind: 'op',
      labelKey: 'roles.op',
      addressHex: toHex(senderAddress(ctx.op.pk)),
      rawTweak: ctx.op.tweak,
    })
  }

  return roles
}

export type PostRole = 'op' | 'mop' | 'thread_mod' | 'thread_admin' | 'board_mod' | 'forum_mod' | 'forum_admin' | null

export function postRole(addr: Uint8Array, mods: Moderators, opAddressHex: string | null): PostRole {
  const hex = toHex(addr)
  if (opAddressHex !== null && hex === opAddressHex) {
    return mods.thread_admin && toHex(mods.thread_admin) === hex ? 'mop' : 'op'
  }
  return roleForAddress(addr, mods)
}

export function roleForAddress(addr: Uint8Array, mods: Moderators): PostRole {
  const hex = toHex(addr)
  if (mods.thread_admin && toHex(mods.thread_admin) === hex) return 'thread_admin'
  if (mods.forum_admin && toHex(mods.forum_admin) === hex) return 'forum_admin'
  const inList = (l: Uint8Array[]) => l.some(a => toHex(a) === hex)
  if (inList(mods.thread_mods)) return 'thread_mod'
  if (inList(mods.board_mods)) return 'board_mod'
  if (inList(mods.forum_mods)) return 'forum_mod'
  return null
}

export const ROLE_STYLE: Record<string, { text: string; color: string; bold: boolean }> = {
  op: { text: '#OP', color: '#008000', bold: false },
  mop: { text: '#MOP', color: '#ff6600', bold: false },
  thread_mod: { text: '# THREAD MOD #', color: '#ff6600', bold: false },
  thread_admin: { text: '# THREAD ADMIN #', color: '#cc0000', bold: false },
  board_mod: { text: '# BOARD MOD #', color: '#003366', bold: false },
  forum_mod: { text: '# MOD #', color: '#800080', bold: true },
  forum_admin: { text: '# ADMIN #', color: '#000000', bold: true },
}
