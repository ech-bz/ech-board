import { createContext, useContext, type Accessor, type JSX } from 'solid-js'
import { useMod } from './mod'
import { useModals } from './modals'
import { useStore } from './store'
import { roleForThreadAction, canBan, canBanAny, type BanScope } from '../core/intent/capabilities'
import type { RoleKind, RoleOption } from '../core/intent/roles'
import type { EntityNs } from '../core/intent/upgrade'
import type { BoardObject, PostObject, PostPartInput, ThreadObject } from '../core/bcs/types'
import { fromHex } from '../core/intent/crypto'

export type ThreadModAction = 'close' | 'open' | 'delete' | 'restore'

export interface EntityActions {
  runThreadAction: (action: ThreadModAction, threadUid: string) => void
  runThreadCloseDelete: (threadUid: string) => void
  runThreadEditTopic: (threadUid: string, topic: string) => void
  runUpgrade: (ns: EntityNs, object: PostObject) => void
  runPostDelete: (post: PostObject, threadUid: string, action: 'delete' | 'restore') => void
  runBatchPostDelete: (posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void) => Promise<void>
  runBatchPostRestore: (posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void) => Promise<void>
  runPostEdit: (
    post: PostObject,
    threadUid: string,
    text: string,
    postMap: Map<number, { id: Uint8Array; author: Uint8Array }>,
    secretRefs: Map<number, PostPartInput>,
  ) => void
  runBan: (post: PostObject, threadUid: string, scope: BanScope, mask: number, durationMs: number, reason: string) => void
  runUnban: (post: PostObject, threadUid: string, scope: BanScope) => void
  runBanMedia: (post: PostObject, threadUid: string, hashes: Uint8Array[]) => void
  runUnbanMedia: (post: PostObject, threadUid: string, hashes: Uint8Array[]) => void
  roleKinds: Accessor<RoleKind[]>
  canThreadClose: Accessor<boolean>
  canThreadOpen: Accessor<boolean>
  canThreadDelete: Accessor<boolean>
  canThreadRestore: Accessor<boolean>
  canThreadEditTopic: Accessor<boolean>
  canThreadPin: Accessor<boolean>
  isThreadPinned: (threadUid: string) => boolean
  toggleThreadPin: (threadUid: string) => void
  threadTopic: (threadUid: string) => string
  threadUid: () => string | null
  selectedPosts: (uids: Set<string>) => PostObject[]
  deletedCount: (uids: Set<string>) => number
  openThreadTopic: (threadUid: string) => void
  openThreadModerators: () => void
  openBoardModerators: () => void
  openForumModerators: () => void
  openBoardSettings: () => void
  openBan: (posts: PostObject[], threadUid: string, deletePosts?: boolean) => void
  findAuthorPosts: (ref: PostObject, posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void) => Promise<string[]>
  openBanUid: () => void
  openBans: (title: string, uid: string) => void
  openLogs: (tab: 'forum' | 'board' | 'thread' | 'post', object?: BoardObject | PostObject | ThreadObject) => void
  canBanAny: Accessor<boolean>
  canBanScope: (scope: BanScope) => boolean
  busy: Accessor<boolean>
}

const EntityActionsContext = createContext<EntityActions>()

export function EntityActionsProvider(props: { roles: RoleOption[]; children: JSX.Element }) {
  const store = useStore()
  const mod = useMod()
  const modals = useModals()
  const has = (action: Parameters<typeof roleForThreadAction>[0]) => () => !!roleForThreadAction(action, props.roles)
  const value: EntityActions = {
    runThreadAction: (action, threadUid) => mod.runThreadAction(action, threadUid),
    runThreadCloseDelete: (threadUid) => mod.runThreadCloseDelete(threadUid),
    runThreadEditTopic: (threadUid, topic) => mod.runThreadEditTopic(threadUid, topic),
    runUpgrade: (ns, object) => mod.runUpgrade(ns, object),
    runPostDelete: (post, threadUid, action) => mod.runPostDelete(post, action, threadUid),
    runBatchPostDelete: (posts, threadUid, onProgress) => mod.runBatchPostDelete(posts, threadUid, onProgress),
    runBatchPostRestore: (posts, threadUid, onProgress) => mod.runBatchPostRestore(posts, threadUid, onProgress),
    runPostEdit: (post, threadUid, text, postMap, secretRefs) => mod.runPostEdit(post, threadUid, text, postMap, secretRefs),
    runBan: (post, threadUid, scope, mask, durationMs, reason) => mod.runBan(post, threadUid, scope, mask, durationMs, reason),
    runUnban: (post, threadUid, scope) => mod.runUnban(post, threadUid, scope),
    runBanMedia: (post, threadUid, hashes) => mod.runPostBanMedia(post, threadUid, hashes),
    runUnbanMedia: (post, threadUid, hashes) => mod.runPostUnbanMedia(post, threadUid, hashes),
    roleKinds: () => props.roles.map((role) => role.kind),
    canThreadClose: has('close'),
    canThreadOpen: has('open'),
    canThreadDelete: has('delete'),
    canThreadRestore: has('restore'),
    canThreadEditTopic: has('edit_topic'),
    canThreadPin: has('pin'),
    isThreadPinned: (threadUid) => store.pinnedUids().includes(threadUid),
    toggleThreadPin: (threadUid) => {
      const current = store.pinnedUids()
      const next = current.includes(threadUid)
        ? current.filter((uid) => uid !== threadUid).map((uid) => fromHex(uid))
        : [...current.map((uid) => fromHex(uid)), fromHex(threadUid)]
      const info = store.info()
      if (info) void mod.runBoardSetPinned(next, info.boardUid)
    },
    threadTopic: (threadUid) => store.threadTopic(threadUid),
    threadUid: () => store.threadUid(),
    selectedPosts: (uids) => {
      const posts: PostObject[] = []
      for (const uid of uids) {
        const record = store.get(uid)
        if (record) posts.push(record.post)
      }
      return posts
    },
    deletedCount: (uids) => {
      let count = 0
      for (const uid of uids) if (store.get(uid)?.deleted) count++
      return count
    },
    openThreadTopic: (threadUid) => modals.openThreadTopic(threadUid),
    openThreadModerators: () => modals.openThreadModerators(),
    openBoardModerators: () => modals.openBoardModerators(),
    openForumModerators: () => modals.openForumModerators(),
    openBoardSettings: () => modals.openBoardSettings(),
    openBan: (posts, threadUid, deletePosts) => modals.openBan(posts, threadUid, deletePosts),
    findAuthorPosts: (ref, posts, threadUid, onProgress) => mod.findAuthorPosts(ref, posts, threadUid, onProgress),
    openBanUid: () => modals.openBanUid(),
    openBans: (title, uid) => modals.openBans(title, uid),
    openLogs: (tab, object) => modals.openLogs(tab, object as never),
    canBanAny: () => canBanAny(props.roles.map((role) => role.kind)),
    canBanScope: (scope) => canBan(scope, props.roles.map((role) => role.kind)),
    busy: mod.busy,
  }
  return <EntityActionsContext.Provider value={value}>{props.children}</EntityActionsContext.Provider>
}

export function useEntityActions(): EntityActions {
  const value = useContext(EntityActionsContext)
  if (!value) throw new Error('useEntityActions outside provider')
  return value
}
