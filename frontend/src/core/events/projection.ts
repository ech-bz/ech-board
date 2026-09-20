import type {
  PostObject, ThreadObject, BoardObject, ForumObject,
  PostProjectionData, ThreadProjectionData, BoardProjectionData, ForumProjectionData,
} from '../bcs/types'

type ProjVariant<T> = Extract<T[keyof T], object>
export type PostProjectionVariant = ProjVariant<PostProjectionData>
export type ThreadProjectionVariant = ProjVariant<ThreadProjectionData>
export type BoardProjectionVariant = ProjVariant<BoardProjectionData>
export type ForumProjectionVariant = ProjVariant<ForumProjectionData>

export function mapProjection<T extends object>(proj: T, mutate: (d: ProjVariant<T>) => T[keyof T]): T {
  const variant = Object.keys(proj)[0] as keyof T
  return { [variant]: mutate(proj[variant] as ProjVariant<T>) } as T
}

export function mapThread(thread: ThreadObject, mutate: (d: ThreadProjectionVariant) => ThreadProjectionData[keyof ThreadProjectionData]): ThreadObject {
  return { ...thread, projection: mapProjection(thread.projection, mutate) }
}

export function mapPost(post: PostObject, mutate: (d: PostProjectionVariant) => PostProjectionData[keyof PostProjectionData]): PostObject {
  return { ...post, projection: mapProjection(post.projection, mutate) }
}

export function mapBoard(board: BoardObject, mutate: (d: BoardProjectionVariant) => BoardProjectionData[keyof BoardProjectionData]): BoardObject {
  return { ...board, projection: mapProjection(board.projection, mutate) }
}

export function mapForum(forum: ForumObject, mutate: (d: ForumProjectionVariant) => ForumProjectionData[keyof ForumProjectionData]): ForumObject {
  return { ...forum, projection: mapProjection(forum.projection, mutate) }
}

export function clonePairs(list: [Uint8Array, string][] | undefined): [Uint8Array, string][] {
  return (list ?? []).map(([h, c]) => [h, c] as [Uint8Array, string])
}

export function upgradePost(proj: PostProjectionData, to: number): PostProjectionData {
  const v = Object.keys(proj)[0] as keyof PostProjectionData
  let cur: number
  switch (v) {
    case 'V1': cur = 1; break
    case 'V2': cur = 2; break
    case 'V3': cur = 3; break
    default: throw new Error(`unknown post projection variant ${v}`)
  }
  if (to <= cur) return proj
  let next: Record<string, unknown> = { ...(proj[v] as object) }
  if (cur < 2 && to >= 2) next = { ...next, multi_vote: false }
  if (cur < 3 && to >= 3) next = { ...next, banned_media: [] }
  switch (to) {
    case 2: return { V2: next } as unknown as PostProjectionData
    case 3: return { V3: next } as unknown as PostProjectionData
    default: throw new Error(`unsupported post projection version ${to}`)
  }
}

export function upgradeThread(proj: ThreadProjectionData, to: number): ThreadProjectionData {
  const v = Object.keys(proj)[0] as keyof ThreadProjectionData
  let cur: number
  switch (v) {
    case 'V1': cur = 1; break
    case 'V2': cur = 2; break
    case 'V3': cur = 3; break
    default: throw new Error(`unknown thread projection variant ${v}`)
  }
  if (to <= cur) return proj
  let next: Record<string, unknown> = { ...(proj[v] as object) }
  if (cur < 3 && to >= 3) next = { ...next, posts_deleted: '0' }
  switch (to) {
    case 2: return { V2: next } as unknown as ThreadProjectionData
    case 3: return { V3: next } as unknown as ThreadProjectionData
    default: throw new Error(`unsupported thread projection version ${to}`)
  }
}

export function upgradeBoard(proj: BoardProjectionData, to: number): BoardProjectionData {
  const v = Object.keys(proj)[0] as keyof BoardProjectionData
  let cur: number
  switch (v) {
    case 'V1': cur = 1; break
    case 'V2': cur = 2; break
    default: throw new Error(`unknown board projection variant ${v}`)
  }
  if (to <= cur) return proj
  let next: Record<string, unknown> = { ...(proj[v] as object) }
  if (cur < 2 && to >= 2) next = { ...next, pinned: [] }
  switch (to) {
    case 2: return { V2: next } as unknown as BoardProjectionData
    default: throw new Error(`unsupported board projection version ${to}`)
  }
}
