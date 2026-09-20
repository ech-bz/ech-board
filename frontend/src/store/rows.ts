import { BoardProjection, PostProjection, ThreadProjection, type PostObject, type ThreadObject } from '../core/bcs/types'
import { deriveThreadTitle, authorAddressOf, postUid, threadCount, threadTopicText } from '../core/posts'
import { toHex } from '../core/intent/crypto'
import type { BoardViewData } from '../core/load'

export interface ThreadRowRecord {
  readonly uid: string
  readonly number: number
  readonly opUid: string | null
  readonly replyUids: readonly string[]
  readonly title: string
  readonly topic: string
  readonly count: number
  readonly closed: boolean
  readonly pinned: boolean
  readonly deleted: boolean
  readonly opAdmin: boolean
  readonly skipped: number
  readonly thread: ThreadObject
}

export interface RowsResult {
  all: ThreadRowRecord[]
  live: ThreadRowRecord[]
  relNum: Map<string, number>
}

function rowOf(data: BoardViewData, thread: ThreadObject, tp: ThreadProjection, boardPinned: Set<string>, replies: readonly PostObject[]): ThreadRowRecord {
  const uid = toHex(thread.root.id)
  const op = data.opPosts.get(uid) ?? null
  const count = threadCount(tp)
  const admin = tp.admin()
  return {
    uid,
    number: Number(tp.number()),
    opUid: op ? toHex(op.root.id) : null,
    replyUids: replies.map((p) => postUid(p)),
    title: deriveThreadTitle(tp, data.contentMap, op),
    topic: threadTopicText(tp, data.contentMap),
    count,
    closed: tp.closed(),
    pinned: tp.pinned() || boardPinned.has(uid),
    deleted: tp.deleted(),
    opAdmin: admin !== null && op !== null && toHex(admin) === authorAddressOf(op),
    skipped: Math.max(0, count - 1 - replies.length),
    thread,
  }
}

export function buildRows(data: BoardViewData): RowsResult {
  const boardPinned = new Set(new BoardProjection(data.board.projection).pinned().map(toHex))
  const all: ThreadRowRecord[] = []
  const live: ThreadRowRecord[] = []
  const relNum = new Map<string, number>()
  for (const thread of data.threads) {
    const tp = new ThreadProjection(thread.projection)
    const uid = toHex(thread.root.id)
    const repliesAll = data.last3.get(uid) ?? []
    const repliesLive = repliesAll.filter((p) => !new PostProjection(p.projection).deleted())
    const rowAll = rowOf(data, thread, tp, boardPinned, repliesAll)
    all.push(rowAll)
    live.push(repliesAll.length === repliesLive.length ? rowAll : rowOf(data, thread, tp, boardPinned, repliesLive))
    if (rowAll.opUid) relNum.set(rowAll.opUid, 1)
    for (let i = 0; i < rowAll.replyUids.length; i++) relNum.set(rowAll.replyUids[i], rowAll.count - rowAll.replyUids.length + 1 + i)
  }
  return { all, live, relNum }
}

export function sameRow(x: ThreadRowRecord, y: ThreadRowRecord): boolean {
  if (x === y) return true
  if (x.uid !== y.uid || x.number !== y.number || x.opUid !== y.opUid || x.title !== y.title || x.topic !== y.topic || x.count !== y.count || x.closed !== y.closed || x.pinned !== y.pinned || x.deleted !== y.deleted || x.opAdmin !== y.opAdmin || x.skipped !== y.skipped) return false
  if (x.replyUids.length !== y.replyUids.length) return false
  for (let j = 0; j < x.replyUids.length; j++) if (x.replyUids[j] !== y.replyUids[j]) return false
  return true
}

export function sameRows(a: readonly ThreadRowRecord[], b: readonly ThreadRowRecord[]): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) {
    if (!sameRow(a[i], b[i])) return false
  }
  return true
}
