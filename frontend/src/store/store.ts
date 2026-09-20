import { BoardProjection, PostProjection, ThreadProjection, PostView as PostViewBcs, type BoardObject, type ModeratorsData, type PostObject, type PostPartInput, type ThreadObject } from '../core/bcs/types'
import { MediaMeta } from '../core/bcs/relay'
import { applyBoardBatch, applyThreadBatch, snapshotBoard, snapshotThread, type Batch } from '../core/events/batch'
import { decodeRealtimeEvent } from '../core/events/core'
import { contentUrl, clearMediaCache, type MediaInfo } from '../core/media'
import { contentMapOf, loadBoardPage, mediaMetaMapOf, type BoardViewData, type PostCtx, type ThreadViewData, type TooltipPost } from '../core/load'
import { fetchPostView } from '../core/api/relay'
import { deriveThreadTitle, postMeta, postParts, postReplyTargets, postUid, threadTopicText, type PostRef } from '../core/posts'
import { computeTweak, derivePostKeypair, fromHex, toHex } from '../core/intent/crypto'
import type { DecryptContext } from '../core/intent/decrypt'
import { buildDecryptContext } from '../core/intent/decrypt'
import type { RoleKind } from '../core/intent/roles'
import { getSecretKeyBytes } from '../core/keys/storage'
import { buildPostRecord, missingBlobs, type PostRecord } from './records'
import { buildRows, sameRow, sameRows, type ThreadRowRecord } from './rows'
import { DeadlineScheduler } from './scheduler'

export type StoreMode = 'thread' | 'board'

export interface ThreadPageInfo {
  uid: string
  closed: boolean
  deleted: boolean
  pinned: boolean
}

export interface BoardPageInfo {
  uid: string
  description: string
  nextCursor: number | null
}

export interface StaticInfo {
  relayUrl: string
  forumId: string
  boardUid: string
  nonceShardsId: string
  moderators: ModeratorsData
  boardReactions: Uint8Array[]
}

type Listener = () => void

function sameList(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
  return true
}

function sameItems<T>(a: readonly T[], b: readonly T[]): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
  return true
}

function replaceInMap(map: Map<string, PostObject>, uid: string, post: PostObject): Map<string, PostObject> {
  if (!map.has(uid)) return map
  const next = new Map(map)
  next.set(uid, post)
  return next
}

function replaceInListMap(map: Map<string, PostObject[]>, uid: string, post: PostObject): Map<string, PostObject[]> {
  let touched = false
  const next = new Map<string, PostObject[]>()
  for (const [key, list] of map) {
    if (!list.some((item) => postUid(item) === uid)) {
      next.set(key, list)
      continue
    }
    next.set(key, list.map((item) => (postUid(item) === uid ? post : item)))
    touched = true
  }
  return touched ? next : map
}

export class ForumStore {
  private mode: StoreMode | null = null
  private ctx: PostCtx | null = null
  private threadData: ThreadViewData | null = null
  private boardData: BoardViewData | null = null
  private staticInfo: StaticInfo | null = null
  private threadPage: ThreadPageInfo | null = null
  private boardPage: BoardPageInfo | null = null
  private postsByUid = new Map<string, PostObject>()
  private threadOf = new Map<string, string>()
  private order: string[] = []
  private ordinals = new Map<string, number>()
  private relNumOf = new Map<string, number>()
  private records = new Map<string, PostRecord>()
  private listeners = new Map<string, Set<Listener>>()
  private postListeners = new Set<Listener>()
  private rowListeners = new Set<Listener>()
  private galleryListeners = new Set<Listener>()
  private pageListeners = new Set<Listener>()
  private applied = new Map<string, number>()
  private loadingMore = false
  private scheduler = new DeadlineScheduler()
  private targets = new Map<string, string[]>()
  private backrefs = new Map<string, PostRef[]>()
  private pendingReactions = new Map<string, { local: string | null; base: string | null }>()
  private reactionListeners = new Set<Listener>()
  private liveOrder: string[] = []
  private allOrder: string[] = []
  private liveRows: ThreadRowRecord[] = []
  private allRows: ThreadRowRecord[] = []
  private liveRowsByUid = new Map<string, ThreadRowRecord>()
  private allRowsByUid = new Map<string, ThreadRowRecord>()
  private liveGallery: MediaInfo[] = []
  private allGallery: MediaInfo[] = []
  private roleKinds: RoleKind[] = []
  private master = getSecretKeyBytes()
  private attempted = new Set<string>()

  loadThread(data: ThreadViewData): void {
    this.mode = 'thread'
    this.threadData = data
    this.boardData = null
    this.ctx = data
    this.master = getSecretKeyBytes()
    this.applied = snapshotThread(data)
    this.staticInfo = {
      relayUrl: data.relayUrl,
      forumId: data.forumId,
      boardUid: data.boardUid,
      nonceShardsId: data.nonceShardsId,
      moderators: data.moderators,
      boardReactions: data.boardReactions,
    }
    this.records = new Map()
    this.postsByUid = new Map()
    this.threadOf = new Map()
    this.targets = new Map()
    this.backrefs = new Map()
    this.attempted = new Set()
    this.order = []
    clearMediaCache()
    for (const post of data.posts) {
      const uid = postUid(post)
      this.postsByUid.set(uid, post)
      this.threadOf.set(uid, data.threadUid)
      this.order.push(uid)
    }
    this.recomputeOrdinals()
    this.updateTargets(this.order)
    this.rebuildRecords([...this.order, ...this.backrefs.keys()])
    this.recomputePage()
  }

  loadBoard(data: BoardViewData): void {
    this.mode = 'board'
    this.boardData = data
    this.threadData = null
    this.ctx = data
    this.master = getSecretKeyBytes()
    this.applied = snapshotBoard(data)
    this.staticInfo = {
      relayUrl: data.relayUrl,
      forumId: data.forumId,
      boardUid: data.boardUid,
      nonceShardsId: data.nonceShardsId,
      moderators: data.moderators,
      boardReactions: data.boardReactions,
    }
    this.records = new Map()
    this.postsByUid = new Map()
    this.threadOf = new Map()
    this.targets = new Map()
    this.backrefs = new Map()
    this.attempted = new Set()
    this.order = []
    clearMediaCache()
    for (const [threadUid, post] of data.opPosts) {
      const uid = postUid(post)
      this.postsByUid.set(uid, post)
      this.threadOf.set(uid, threadUid)
    }
    for (const [threadUid, list] of data.last3) {
      for (const post of list) {
        const uid = postUid(post)
        this.postsByUid.set(uid, post)
        this.threadOf.set(uid, threadUid)
      }
    }
    this.recomputeRows()
    const uids = [...this.postsByUid.keys()]
    this.updateTargets(uids)
    this.rebuildRecords([...uids, ...this.backrefs.keys()])
    this.recomputePage()
  }

  setRoleKinds(kinds: RoleKind[]): void {
    if (this.roleKinds.length === kinds.length && this.roleKinds.every((kind, i) => kind === kinds[i])) return
    this.roleKinds = kinds
    this.rebuildRecords([...this.records.keys()])
  }

  applyBatch(batch: Batch): void {
    if (this.mode === 'thread') this.applyThread(batch)
    else if (this.mode === 'board') this.applyBoard(batch)
  }

  get(uid: string): PostRecord | undefined {
    return this.records.get(uid)
  }

  subscribe(uid: string, listener: Listener): () => void {
    let set = this.listeners.get(uid)
    if (!set) {
      set = new Set()
      this.listeners.set(uid, set)
    }
    set.add(listener)
    return () => {
      const current = this.listeners.get(uid)
      if (!current) return
      current.delete(listener)
      if (current.size === 0) this.listeners.delete(uid)
    }
  }

  postsOf(showDeleted: boolean): readonly string[] {
    return showDeleted ? this.allOrder : this.liveOrder
  }

  subscribePosts(listener: Listener): () => void {
    this.postListeners.add(listener)
    return () => this.postListeners.delete(listener)
  }

  rowsOf(showDeleted: boolean): readonly ThreadRowRecord[] {
    return showDeleted ? this.allRows : this.liveRows
  }

  subscribeRows(listener: Listener): () => void {
    this.rowListeners.add(listener)
    return () => this.rowListeners.delete(listener)
  }

  galleryOf(showDeleted: boolean): readonly MediaInfo[] {
    return showDeleted ? this.allGallery : this.liveGallery
  }

  subscribeGallery(listener: Listener): () => void {
    this.galleryListeners.add(listener)
    return () => this.galleryListeners.delete(listener)
  }

  info(): StaticInfo | null {
    return this.staticInfo
  }

  decryptCtx(): DecryptContext | null {
    return this.ctx ? this.ctx.decryptCtx : null
  }

  page(): ThreadPageInfo | null {
    return this.threadPage
  }

  pinnedUids(): string[] {
    const board = this.boardData
    if (!board) return []
    const pinned = new BoardProjection(board.board.projection).pinned()
    return (pinned as Uint8Array[] | undefined)?.map((uid) => toHex(uid)) ?? []
  }

  threadTopic(threadUid: string): string {
    const threadData = this.threadData
    if (threadData && threadData.threadUid === threadUid) {
      return threadTopicText(new ThreadProjection(threadData.thread.projection), threadData.contentMap)
    }
    const board = this.boardData
    if (!board) return ''
    const thread = board.threads.find((item) => toHex(item.root.id) === threadUid)
    return thread ? threadTopicText(new ThreadProjection(thread.projection), board.contentMap) : ''
  }

  threadTitle(): string {
    const data = this.threadData
    if (!data) return ''
    const tp = new ThreadProjection(data.thread.projection)
    const opUid = toHex(tp.op())
    const op = data.posts.find((post) => toHex(post.root.id) === opUid)
    return deriveThreadTitle(tp, data.contentMap, op)
  }

  content(): Map<string, Uint8Array> | null {
    return this.ctx ? this.ctx.contentMap : null
  }

  boardObject(): BoardObject | null {
    return this.boardData ? this.boardData.board : null
  }

  threadObject(): ThreadObject | null {
    return this.threadData ? this.threadData.thread : null
  }

  tooltipPost(uid: string, slug: string): TooltipPost | null {
    const refs = this.backrefs.get(uid) ?? []
    const thread = this.threadData
    if (thread) {
      const post = thread.posts.find((item) => postUid(item) === uid)
      return post ? { post, threadUid: thread.threadUid, ctx: thread, threadObj: thread.thread, slug, refs } : null
    }
    const board = this.boardData
    if (!board) return null
    const op = [...board.opPosts.entries()].find(([, item]) => postUid(item) === uid)
    if (op) return { post: op[1], threadUid: op[0], ctx: board, threadObj: board.threads.find((item) => toHex(item.root.id) === op[0]), slug, refs }
    for (const [threadUid, list] of board.last3) {
      const post = list.find((item) => postUid(item) === uid)
      if (post) return { post, threadUid, ctx: board, threadObj: board.threads.find((item) => toHex(item.root.id) === threadUid), slug, refs }
    }
    return null
  }

  boardModerators(): Uint8Array[] {
    return this.boardData ? this.boardData.moderators.board_mods : []
  }

  postMap(): Map<number, { id: Uint8Array; author: Uint8Array }> {
    const map = new Map<number, { id: Uint8Array; author: Uint8Array }>()
    for (const uid of this.order) {
      const post = this.postsByUid.get(uid)
      if (!post) continue
      const pp = new PostProjection(post.projection)
      map.set(Number(pp.number()), { id: post.root.id, author: pp.sender().pk })
    }
    return map
  }

  threadUid(): string | null {
    const first = this.order[0]
    return first ? (this.threadOf.get(first) ?? null) : null
  }

  boardInfo(): BoardPageInfo | null {
    return this.boardPage
  }

  loadingMoreThreads(): boolean {
    return this.loadingMore
  }

  async loadMoreThreads(): Promise<void> {
    const current = this.boardData
    if (!current || current.nextCursor === null || this.loadingMore) return
    this.loadingMore = true
    this.notifyPage()
    try {
      const page = await loadBoardPage(current.relayUrl, current.boardUid, current.nextCursor)
      const latest = this.boardData
      if (!latest || latest.boardUid !== current.boardUid) return
      const seen = new Set(latest.threads.map((thread) => toHex(thread.root.id)))
      const threads = [...latest.threads, ...page.threads.filter((thread) => !seen.has(toHex(thread.root.id)))]
      const opPosts = new Map(latest.opPosts)
      for (const [uid, post] of page.opPosts) opPosts.set(uid, post)
      const last3 = new Map(latest.last3)
      for (const [uid, list] of page.last3) last3.set(uid, list)
      const contentMap = new Map(latest.contentMap)
      for (const [hash, blob] of page.contentMap) contentMap.set(hash, blob)
      const mediaMeta = new Map(latest.mediaMeta)
      for (const [hash, meta] of page.mediaMeta) mediaMeta.set(hash, meta)
      const posts = [...opPosts.values(), ...Array.from(last3.values()).flat()]
      const myReactions = new Map(latest.myReactions)
      for (const post of posts) myReactions.delete(toHex(post.root.id))
      for (const [uid, hash] of page.myReactions) myReactions.set(uid, hash)
      const next: BoardViewData = {
        ...latest,
        threads,
        opPosts,
        last3,
        contentMap,
        mediaMeta,
        myReactions,
        decryptCtx: buildDecryptContext(posts, this.master, fromHex(latest.forumId)),
        nextCursor: page.nextCursor,
      }
      this.applied = snapshotBoard(next)
      this.ingestBoard(next)
    } finally {
      this.loadingMore = false
      this.notifyPage()
    }
  }

  subscribePage(listener: Listener): () => void {
    this.pageListeners.add(listener)
    return () => this.pageListeners.delete(listener)
  }

  async refetchPost(uid: string, myReaction?: string | null): Promise<void> {
    const ctx = this.ctx
    if (!ctx) return
    const bytes = await fetchPostView(ctx.relayUrl, uid)
    const view = PostViewBcs.parse(new Uint8Array(bytes))
    const content = contentMapOf([...view.text, ...view.plain_text])
    const meta = mediaMetaMapOf(view.media_meta)
    const threadData = this.threadData
    const boardData = this.boardData
    if (threadData) {
      const contentMap = content.size > 0 ? new Map([...threadData.contentMap, ...content]) : threadData.contentMap
      const mediaMeta = meta.size > 0 ? new Map([...threadData.mediaMeta, ...meta]) : threadData.mediaMeta
      const myReactions = this.withReaction(threadData.myReactions, uid, myReaction)
      const data: ThreadViewData = {
        ...threadData,
        contentMap,
        mediaMeta,
        myReactions,
        posts: threadData.posts.map((post) => (postUid(post) === uid ? view.post : post)),
      }
      this.threadData = data
      this.ctx = data
      this.postsByUid.set(uid, view.post)
      this.rebuildRecords([uid])
      this.hydrate([uid])
      return
    }
    if (!boardData) return
    const contentMap = content.size > 0 ? new Map([...boardData.contentMap, ...content]) : boardData.contentMap
    const mediaMeta = meta.size > 0 ? new Map([...boardData.mediaMeta, ...meta]) : boardData.mediaMeta
    const myReactions = this.withReaction(boardData.myReactions, uid, myReaction)
    const threadUid = this.threadOf.get(uid)
    const data: BoardViewData = {
      ...boardData,
      contentMap,
      mediaMeta,
      myReactions,
      opPosts: replaceInMap(boardData.opPosts, uid, view.post),
      last3: replaceInListMap(boardData.last3, uid, view.post),
    }
    this.boardData = data
    this.ctx = data
    this.postsByUid.set(uid, view.post)
    if (threadUid && data.opPosts.get(threadUid) === view.post) this.recomputeRows()
    this.rebuildRecords([uid])
    this.hydrate([uid])
  }

  private withReaction(current: Map<string, string>, uid: string, myReaction?: string | null): Map<string, string> {
    if (myReaction === undefined) return current
    const next = new Map(current)
    if (myReaction) next.set(uid, myReaction)
    else next.delete(uid)
    return next
  }

  pendingReactionOf(uid: string): { local: string | null; base: string | null } | null {
    return this.pendingReactions.get(uid) ?? null
  }

  subscribeReactions(listener: Listener): () => void {
    this.reactionListeners.add(listener)
    return () => this.reactionListeners.delete(listener)
  }

  setPendingReaction(uid: string, local: string | null, base: string | null): void {
    this.pendingReactions.set(uid, { local, base })
    this.rebuildRecords([uid])
    this.notifyReactions()
  }

  clearPendingReaction(uid: string): void {
    if (!this.pendingReactions.delete(uid)) return
    this.rebuildRecords([uid])
    this.notifyReactions()
  }

  private displayReaction(uid: string): { local: string | null; base: string | null } {
    const base = this.ctx ? this.ctx.myReactions.get(uid) ?? null : null
    const pending = this.pendingReactions.get(uid)
    if (!pending) return { local: base, base }
    if (pending.base !== base) {
      this.pendingReactions.delete(uid)
      return { local: base, base }
    }
    return { local: pending.local, base }
  }

  private notifyReactions(): void {
    for (const listener of this.reactionListeners) listener()
  }

  addContent(hashHex: string, blob: Uint8Array): void {    const data = this.threadData ?? this.boardData
    if (!data) return
    const contentMap = new Map(data.contentMap)
    contentMap.set(hashHex, blob)
    if (this.threadData) {
      const next: ThreadViewData = { ...this.threadData, contentMap }
      this.threadData = next
      this.ctx = next
    } else if (this.boardData) {
      const next: BoardViewData = { ...this.boardData, contentMap }
      this.boardData = next
      this.ctx = next
    }
    this.rebuildRecords([...this.records.keys()])
  }

  dispose(): void {
    this.scheduler.dispose()
    this.listeners.clear()
    this.postListeners.clear()
    this.rowListeners.clear()
    this.galleryListeners.clear()
    this.pageListeners.clear()
  }

  private applyThread(batch: Batch): void {
    const prev = this.threadData
    if (!prev) return
    const data = applyThreadBatch(prev, batch, this.applied).data
    this.threadData = data
    this.ctx = data
    const changed = new Set<string>()
    for (let i = 0; i < data.posts.length; i++) {
      const post = data.posts[i]
      if (i >= prev.posts.length || prev.posts[i] !== post) {
        const uid = postUid(post)
        this.postsByUid.set(uid, post)
        this.threadOf.set(uid, data.threadUid)
        changed.add(uid)
      }
    }
    const lengthChanged = data.posts.length !== prev.posts.length
    if (lengthChanged) this.order = data.posts.map(postUid)
    const reactions = this.reactionDelta(data, batch)
    if (reactions) {
      data.myReactions = reactions.map
      for (const uid of reactions.changed) changed.add(uid)
    }
    if (lengthChanged || this.deletedSignature(data) !== this.deletedSignature(prev)) this.recomputeOrdinals()
    for (const uid of this.updateTargets(changed)) changed.add(uid)
    this.rebuildRecords(changed)
    if (lengthChanged) this.recomputePage()
    if (data.decryptCtx !== prev.decryptCtx) this.notifyPage()
    this.hydrate(changed)
  }

  private applyBoard(batch: Batch): void {
    const prev = this.boardData
    if (!prev) return
    this.ingestBoard(applyBoardBatch(prev, batch, this.applied).data)
  }

  private ingestBoard(data: BoardViewData): void {
    const prev = this.boardData
    if (!prev) return
    this.boardData = data
    this.ctx = data
    const changed = new Set<string>()
    const structural = data.threads.length !== prev.threads.length
    for (let i = 0; i < data.threads.length; i++) {
      const thread = data.threads[i]
      if (i >= prev.threads.length || prev.threads[i] !== thread) {
        const uid = toHex(thread.root.id)
        const op = data.opPosts.get(uid)
        if (op) {
          const opUid = postUid(op)
          this.postsByUid.set(opUid, op)
          this.threadOf.set(opUid, uid)
          changed.add(opUid)
        }
        for (const reply of data.last3.get(uid) ?? []) {
          const replyUid = postUid(reply)
          if (prev.last3.get(uid)?.some((p) => postUid(p) === replyUid)) continue
          this.postsByUid.set(replyUid, reply)
          this.threadOf.set(replyUid, uid)
          changed.add(replyUid)
        }
      }
    }
    for (const [threadUid, op] of data.opPosts) {
      if (prev.opPosts.get(threadUid) === op) continue
      const uid = postUid(op)
      this.postsByUid.set(uid, op)
      this.threadOf.set(uid, threadUid)
      changed.add(uid)
    }
    for (const [threadUid, list] of data.last3) {
      const before = prev.last3.get(threadUid) ?? []
      for (let i = 0; i < list.length; i++) {
        if (i < before.length && before[i] === list[i]) continue
        const uid = postUid(list[i])
        this.postsByUid.set(uid, list[i])
        this.threadOf.set(uid, threadUid)
        changed.add(uid)
      }
    }
    this.recomputeRows()
    for (const [uid, relNum] of this.relNumOf) {
      const record = this.records.get(uid)
      if (record && record.relNum !== relNum) changed.add(uid)
    }
    for (const uid of this.updateTargets(changed)) changed.add(uid)
    this.rebuildRecords(changed)
    if (structural || data.nextCursor !== prev.nextCursor || data.board !== prev.board) this.recomputeBoardPage(data)
    if (data.decryptCtx !== prev.decryptCtx) this.notifyPage()
    this.hydrate(changed)
  }

  private partsOf(post: PostObject): PostPartInput[] | null {
    const ctx = this.ctx
    if (!ctx) return []
    const hash = postMeta(post).textHash
    if (hash === null) return []
    if (!ctx.contentMap.has(hash)) return null
    return postParts(post, ctx.contentMap)
  }

  private recomputeOrdinals(): void {
    const data = this.threadData
    if (!data) return
    let ord = 0
    this.ordinals = new Map()
    for (const post of data.posts) {
      const meta = postMeta(post)
      if (!meta.deleted) ord += 1
      this.ordinals.set(meta.uid, ord)
    }
  }

  private recomputeRows(): void {
    const data = this.boardData
    if (!data) return
    const result = buildRows(data)
    this.relNumOf = result.relNum
    const all = this.reuseRows(result.all, this.allRowsByUid)
    const live = this.reuseRows(result.live, this.liveRowsByUid)
    this.allRowsByUid = new Map(all.map((row) => [row.uid, row]))
    this.liveRowsByUid = new Map(live.map((row) => [row.uid, row]))
    if (!sameRows(this.allRows, all)) {
      this.allRows = all
      this.liveRows = live
      this.notifyRows()
    } else if (!sameRows(this.liveRows, live)) {
      this.liveRows = live
      this.notifyRows()
    }
  }

  private reuseRows(next: readonly ThreadRowRecord[], prev: Map<string, ThreadRowRecord>): ThreadRowRecord[] {
    return next.map((row) => {
      const old = prev.get(row.uid)
      return old && sameRow(old, row) ? old : row
    })
  }

  private updateTargets(uids: Iterable<string>): Set<string> {
    const affected = new Set<string>()
    for (const uid of uids) {
      const post = this.postsByUid.get(uid)
      if (!post) continue
      const parts = this.partsOf(post)
      const next = parts ? postReplyTargets(parts) : []
      const prev = this.targets.get(uid) ?? []
      if (sameList(prev, next)) continue
      this.targets.set(uid, next)
      const ref: PostRef = { postUid: uid, number: postMeta(post).number }
      for (const target of prev) {
        if (next.includes(target)) continue
        const list = this.backrefs.get(target)
        if (!list) continue
        const filtered = list.filter((r) => r.postUid !== uid)
        if (filtered.length === 0) this.backrefs.delete(target)
        else this.backrefs.set(target, filtered)
        affected.add(target)
      }
      for (const target of next) {
        if (prev.includes(target)) continue
        const list = this.backrefs.get(target) ?? []
        this.backrefs.set(target, [...list, ref].sort((a, b) => a.number - b.number))
        affected.add(target)
      }
    }
    return affected
  }

  private rebuildRecords(uids: Iterable<string>): void {
    const ctx = this.ctx
    if (!ctx) return
    const now = Date.now()
    const forumIdBytes = fromHex(ctx.forumId)
    const threadOp = this.threadData ? new ThreadProjection(this.threadData.thread.projection) : null
    const threadOpUid = threadOp ? toHex(threadOp.op()) : null
    const threadDeletedFlag = threadOp ? threadOp.deleted() : false
    for (const uid of uids) {
      const post = this.postsByUid.get(uid)
      if (!post) continue
      const threadUid = this.threadOf.get(uid)
      if (!threadUid) continue
      const op = this.threadData ? uid === threadOpUid : this.isThreadOp(uid, threadUid)
      const opDeleted = this.threadData ? threadDeletedFlag : this.isThreadDeleted(threadUid)
      const relNum = this.threadData ? this.ordinals.get(uid) ?? 0 : this.relNumOf.get(uid) ?? 0
      const reaction = this.displayReaction(uid)
      const record = buildPostRecord({
        post,
        threadUid,
        relayUrl: ctx.relayUrl,
        forumIdBytes,
        boardUid: ctx.boardUid,
        contentMap: ctx.contentMap,
        mediaMeta: ctx.mediaMeta,
        decryptCtx: ctx.decryptCtx,
        moderators: ctx.moderators,
        master: this.master,
        boardReactions: ctx.boardReactions,
        roleKinds: this.roleKinds,
        op,
        opDeleted: op && opDeleted,
        relNum,
        refs: this.backrefs.get(uid) ?? [],
        myReaction: reaction.local,
        myReactionBase: reaction.base,
        now,
      })
      this.records.set(uid, record)
      if (record.deadline === null) this.scheduler.cancel(uid)
      else this.scheduler.schedule(uid, record.deadline, () => this.rebuildRecords([uid]))
      this.notify(uid)
    }
    this.recomputeCollections()
  }

  private isThreadOp(uid: string, threadUid: string): boolean {
    const data = this.boardData
    if (!data) return false
    const op = data.opPosts.get(threadUid)
    return op ? postUid(op) === uid : false
  }

  private isThreadDeleted(threadUid: string): boolean {
    const data = this.boardData
    if (!data) return false
    const thread = data.threads.find((t) => toHex(t.root.id) === threadUid)
    return thread ? new ThreadProjection(thread.projection).deleted() : false
  }

  private recomputeCollections(): void {
    if (this.mode === 'thread') {
      const live: string[] = []
      const all: string[] = []
      for (const uid of this.order) {
        const record = this.records.get(uid)
        if (!record) continue
        if (record.hasText && record.parts === null) continue
        all.push(uid)
        if (!record.deleted) live.push(uid)
      }
      if (!sameList(this.allOrder, all) || !sameList(this.liveOrder, live)) {
        this.allOrder = all
        this.liveOrder = live
        this.notifyPosts()
      }
      this.buildGallery(all)
      return
    }
    const sequence: { uid: string; deleted: boolean }[] = []
    for (const row of this.allRows) {
      if (row.opUid) sequence.push({ uid: row.opUid, deleted: row.deleted })
      for (const uid of row.replyUids) {
        const record = this.records.get(uid)
        if (record) sequence.push({ uid, deleted: record.deleted })
      }
    }
    this.buildGallerySequenced(sequence)
    this.recomputeRows()
  }

  private buildGallery(order: readonly string[]): void {
    const sequence = order.map((uid) => ({ uid, deleted: this.records.get(uid)?.deleted ?? false }))
    this.buildGallerySequenced(sequence)
  }

  private buildGallerySequenced(sequence: readonly { uid: string; deleted: boolean }[]): void {
    const live: MediaInfo[] = []
    const all: MediaInfo[] = []
    for (const item of sequence) {
      const record = this.records.get(item.uid)
      if (!record) continue
      for (const media of record.media) {
        if (media.banned) continue
        all.push(media)
        if (!item.deleted) live.push(media)
      }
    }
    if (!sameItems(this.allGallery, all) || !sameItems(this.liveGallery, live)) {
      this.allGallery = all
      this.liveGallery = live
      this.notifyGallery()
    }
  }

  private recomputePage(): void {
    if (this.mode === 'thread') {
      const data = this.threadData
      if (!data) return
      const tp = new ThreadProjection(data.thread.projection)
      const next: ThreadPageInfo = { uid: data.threadUid, closed: tp.closed(), deleted: tp.deleted(), pinned: tp.pinned() }
      const prev = this.threadPage
      this.threadPage = next
      if (!prev || prev.uid !== next.uid || prev.closed !== next.closed || prev.deleted !== next.deleted || prev.pinned !== next.pinned) this.notifyPage()
      return
    }
    this.recomputeBoardPage(this.boardData)
  }

  private recomputeBoardPage(data: BoardViewData | null): void {
    if (!data) return
    const bp = new BoardProjection(data.board.projection)
    const hash = bp.description_hash()
    const bytes = hash ? data.contentMap.get(toHex(hash)) : undefined
    const next: BoardPageInfo = {
      uid: toHex(data.board.root.id),
      description: bytes ? new TextDecoder().decode(bytes) : '',
      nextCursor: data.nextCursor,
    }
    const prev = this.boardPage
    this.boardPage = next
    if (!prev || prev.uid !== next.uid || prev.description !== next.description || prev.nextCursor !== next.nextCursor) this.notifyPage()
  }

  private deletedSignature(data: ThreadViewData | null): string {
    if (!data) return ''
    let out = ''
    for (const post of data.posts) {
      const meta = postMeta(post)
      if (meta.deleted) out += meta.uid
    }
    return out
  }

  private reactionDelta(ctx: PostCtx, batch: Batch): { map: Map<string, string>; changed: string[] } | null {
    let map: Map<string, string> | null = null
    const changed: string[] = []
    const forumIdBytes = fromHex(ctx.forumId)
    for (const env of batch.envelopes) {
      if (env.kind !== 'post' || !env.event) continue
      let ev
      try {
        ev = decodeRealtimeEvent(env.kind, env.event)
      } catch {
        continue
      }
      if (ev.$kind !== 'set_reaction_v2') continue
      const own = toHex(derivePostKeypair(this.master, forumIdBytes, computeTweak(fromHex(env.target.id))).publicKey)
      if (ev.sender.pk !== own) continue
      if (!map) map = new Map(ctx.myReactions)
      const effective = ev.payload.new === ev.payload.old ? null : ev.payload.new
      if (effective) map.set(env.target.id, effective)
      else map.delete(env.target.id)
      changed.push(env.target.id)
    }
    return map ? { map, changed } : null
  }

  private hydrate(uids: Iterable<string>): void {
    const ctx = this.ctx
    if (!ctx) return
    for (const uid of uids) {
      const post = this.postsByUid.get(uid)
      if (!post) continue
      if (postMeta(post).deleted) continue
      const threadUid = this.threadOf.get(uid)
      if (!threadUid) continue
      const missing = missingBlobs(post, ctx.contentMap, ctx.mediaMeta)
      if (missing.text) this.fetchText(ctx, uid, threadUid, 'text', missing.text)
      if (missing.name) this.fetchText(ctx, uid, threadUid, 'plain_text', missing.name)
      for (const hex of missing.media) this.fetchMedia(ctx, uid, threadUid, hex)
    }
  }

  private fetchText(ctx: PostCtx, uid: string, threadUid: string, kind: 'text' | 'plain_text', hash: string): void {
    const key = `${uid}:${kind}:${hash}`
    if (this.attempted.has(key)) return
    this.attempted.add(key)
    fetch(contentUrl(ctx.relayUrl, ctx.boardUid, threadUid, uid, kind, hash))
      .then((res) => (res.ok ? res.arrayBuffer() : null))
      .then((buf) => {
        if (!buf) return
        ctx.contentMap.set(hash, new Uint8Array(buf))
        this.rebuildRecords([uid, ...this.updateTargets([uid])])
      })
      .catch(() => {})
  }

  private fetchMedia(ctx: PostCtx, uid: string, threadUid: string, hash: string): void {
    const key = `${uid}:media:${hash}`
    if (this.attempted.has(key)) return
    this.attempted.add(key)
    fetch(contentUrl(ctx.relayUrl, ctx.boardUid, threadUid, uid, 'media_meta', hash))
      .then((res) => (res.ok ? res.arrayBuffer() : null))
      .then((buf) => {
        if (!buf) return
        let meta
        try {
          meta = MediaMeta.parse(new Uint8Array(buf))
        } catch {
          return
        }
        ctx.mediaMeta.set(hash, meta)
        this.rebuildRecords([uid])
      })
      .catch(() => {})
  }

  private notify(uid: string): void {
    const set = this.listeners.get(uid)
    if (!set) return
    for (const listener of set) listener()
  }

  private notifyPosts(): void {
    for (const listener of this.postListeners) listener()
  }

  private notifyRows(): void {
    for (const listener of this.rowListeners) listener()
  }

  private notifyGallery(): void {
    for (const listener of this.galleryListeners) listener()
  }

  private notifyPage(): void {
    for (const listener of this.pageListeners) listener()
  }
}
