export interface ModMedia {
  id: string
  name: string
  mime: string
  size: number
  width: number
  height: number
  thumbUrl: string
  fullUrl: string
  hidden: boolean
}

export type ModRoleTag = 'forum_mod' | 'mop' | null

export interface ModPost {
  num: number
  board: string
  threadNum: number
  op: boolean
  authorUid: string
  role: ModRoleTag
  ts: number
  text: string
  replyTo: number | null
  media: ModMedia[]
  deleted: boolean
}

export type ReportStatus = 'open' | 'resolved' | 'ignored'

export interface ModReport {
  id: string
  reporterUid: string
  reason: string
  board: string
  threadNum: number
  postNum: number
  ts: number
  status: ReportStatus
}

export type ModActionKind = 'delete' | 'restore' | 'ban' | 'hide_media' | 'delete_file' | 'close' | 'pin'
export type ModTarget = 'post' | 'user' | 'thread' | 'board' | 'forum'
export type ModPlace = 'forum' | 'board' | 'thread'

export interface ModActionEntry {
  id: string
  modUid: string
  kind: ModActionKind
  target: ModTarget
  place: ModPlace
  board: string | null
  threadNum: number | null
  postNum: number | null
  ts: number
  detail: string
  undone: boolean
  undo?: () => void
}

export interface ModFile {
  id: string
  name: string
  mime: string
  size: number
  width: number
  height: number
  thumbUrl: string
  fullUrl: string
  board: string
  threadNum: number
  postNum: number
  authorUid: string
  ts: number
  deleted: boolean
}

export type ModBanScope = 'thread' | 'board' | 'forum'
