import { createRoot, createSignal } from 'solid-js'
import { makeActions, makeFiles, makePosts, makeReports } from './mockData'
import type { ModActionEntry, ModBanScope, ModFile, ModPost, ModReport, ReportStatus } from './types'

export const ME = 'UID0×C0FFEE…'

export const modData = createRoot(() => {
  const [posts, setPosts] = createSignal<ModPost[]>([])
  const [reports, setReports] = createSignal<ModReport[]>([])
  const [actions, setActions] = createSignal<ModActionEntry[]>([])
  const [files, setFiles] = createSignal<ModFile[]>([])
  let seeded = false
  let counter = 0
  let canModerate = false
  const setCanModerate = (value: boolean) => {
    canModerate = value
  }

  const seed = (boards: string[]) => {
    if (seeded) return
    seeded = true
    const now = Date.now()
    const p = makePosts(boards, now)
    setPosts(p)
    setReports((prev) => [...prev, ...makeReports(p, now)])
    setActions(makeActions(p, now))
    setFiles(makeFiles(boards, now))
  }

  const log = (entry: Omit<ModActionEntry, 'id' | 'modUid' | 'ts' | 'undone'>) =>
    setActions((prev) => [{ ...entry, id: `a${++counter}`, modUid: ME, ts: Date.now(), undone: false }, ...prev])

  const patchPosts = (nums: number[], patch: (post: ModPost) => ModPost) => {
    const set = new Set(nums)
    setPosts((prev) => prev.map((post) => (set.has(post.num) ? patch(post) : post)))
  }

  const postByNum = (num: number) => posts().find((post) => post.num === num)

  const setDeleted = (nums: number[], deleted: boolean) => patchPosts(nums, (post) => (post.deleted === deleted ? post : { ...post, deleted }))

  const deletePosts = (nums: number[]) => {
    if (!canModerate) return
    const live = nums.map(postByNum).filter((post): post is ModPost => !!post && !post.deleted)
    if (!live.length) return
    setDeleted(live.map((post) => post.num), true)
    for (const post of live) {
      log({
        kind: 'delete',
        target: post.op ? 'thread' : 'post',
        place: 'thread',
        board: post.board,
        threadNum: post.threadNum,
        postNum: post.num,
        detail: '',
        undo: () => setDeleted([post.num], false),
      })
    }
  }

  const restorePosts = (nums: number[]) => {
    if (!canModerate) return
    const dead = nums.map(postByNum).filter((post): post is ModPost => !!post && post.deleted)
    if (!dead.length) return
    setDeleted(dead.map((post) => post.num), false)
    for (const post of dead) {
      log({
        kind: 'restore',
        target: 'post',
        place: 'thread',
        board: post.board,
        threadNum: post.threadNum,
        postNum: post.num,
        detail: '',
        undo: () => setDeleted([post.num], true),
      })
    }
  }

  const setMediaHidden = (postNum: number, mediaId: string, hidden: boolean) =>
    patchPosts([postNum], (post) => ({ ...post, media: post.media.map((m) => (m.id === mediaId ? { ...m, hidden } : m)) }))

  const hideMedia = (postNum: number, mediaId: string) => {
    if (!canModerate) return
    const post = postByNum(postNum)
    if (!post) return
    setMediaHidden(postNum, mediaId, true)
    log({
      kind: 'hide_media',
      target: 'post',
      place: 'thread',
      board: post.board,
      threadNum: post.threadNum,
      postNum,
      detail: '',
      undo: () => setMediaHidden(postNum, mediaId, false),
    })
  }

  const banPosts = (targets: ModPost[], scope: ModBanScope, mask: number, reason: string) => {
    if (!canModerate) return
    for (const post of targets) {
      log({
        kind: 'ban',
        target: 'user',
        place: scope === 'thread' ? 'thread' : scope === 'board' ? 'board' : 'forum',
        board: scope === 'forum' ? null : post.board,
        threadNum: scope === 'thread' ? post.threadNum : null,
        postNum: post.num,
        detail: `/${mask}${reason ? ` (${reason})` : ''}`,
      })
    }
  }

  const setReportStatus = (id: string, status: ReportStatus) =>
    canModerate &&
    setReports((prev) => prev.map((report) => (report.id === id ? { ...report, status } : report)))

  const submitReport = (reason: string, target: { board: string; threadNum: number; postNum: number }) =>
    setReports((prev) => [
      { id: `u${Date.now()}-${++counter}`, reporterUid: ME, reason, ...target, ts: Date.now(), status: 'open' },
      ...prev,
    ])

  const setFileDeleted = (id: string, deleted: boolean) =>
    setFiles((prev) => prev.map((file) => (file.id === id ? { ...file, deleted } : file)))

  const deleteFile = (id: string) => {
    if (!canModerate) return
    const file = files().find((item) => item.id === id)
    if (!file || file.deleted) return
    setFileDeleted(id, true)
    log({
      kind: 'delete_file',
      target: 'post',
      place: 'thread',
      board: file.board,
      threadNum: file.threadNum,
      postNum: file.postNum,
      detail: file.name,
      undo: () => setFileDeleted(id, false),
    })
  }

  const undoAction = (id: string) => {
    if (!canModerate) return
    const entry = actions().find((item) => item.id === id)
    if (!entry || entry.undone) return
    entry.undo?.()
    setActions((prev) => prev.map((item) => (item.id === id ? { ...item, undone: true } : item)))
  }

  return { posts, reports, actions, files, seed, setCanModerate, deletePosts, restorePosts, hideMedia, banPosts, setReportStatus, submitReport, deleteFile, undoAction }
})
