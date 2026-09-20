import { getCachedDefaultForumId } from '../core/config'

export type SettingSlug = 'key' | 'appearance'
export type SettingKey = 'Master Key' | 'Appearance'

const SETTINGS: { slug: SettingSlug; key: SettingKey }[] = [
  { slug: 'key', key: 'Master Key' },
  { slug: 'appearance', key: 'Appearance' },
]

export function settingKeyToSlug(key: string): SettingSlug | null {
  const found = SETTINGS.find((s) => s.key === key)
  return found ? found.slug : null
}

export function settingSlugToKey(slug: string): SettingKey | null {
  const found = SETTINGS.find((s) => s.slug === slug)
  return found ? found.key : null
}

export type Route =
  | { page: 'boards'; forumId?: string }
  | { page: 'board'; forumId: string; slug: string }
  | { page: 'thread'; forumId: string; slug: string; threadNum: number; replyTo?: number; scrollTo?: 'top' | 'bottom'; postNum?: number }
  | { page: 'compose'; forumId: string; slug: string }
  | { page: 'settings'; setting?: SettingSlug }

function isDefaultForum(forumId: string): boolean {
  const def = getCachedDefaultForumId()
  if (!def) return false
  const norm = (s: string) => s.replace(/^0x/i, '').toLowerCase()
  const a = norm(forumId)
  const b = norm(def)
  return a === b || a.startsWith(b) || b.startsWith(a)
}

export function boardsPath(forumId: string): string {
  return isDefaultForum(forumId) ? '/' : `/forum/${forumId}`
}

export function boardPath(forumId: string, slug: string): string {
  return isDefaultForum(forumId) ? `/${slug}` : `/forum/${forumId}/${slug}`
}

export function threadPath(forumId: string, slug: string, threadNum: number, postNum?: number): string {
  const base = isDefaultForum(forumId) ? `/${slug}/${threadNum}` : `/forum/${forumId}/${slug}/${threadNum}`
  return postNum != null ? `${base}#${postNum}` : base
}

export function composePath(forumId: string, slug: string): string {
  return `${boardPath(forumId, slug)}/new`
}

export function parseRoute(path: string, defaultForumId: string | null): Route {
  const [pathname, hash] = path.split('#')
  const parts = pathname.split('/').filter(Boolean)
  if (parts.length === 0) return { page: 'boards' }
  if (parts[0] === 'settings') {
    if (parts.length === 1) return { page: 'settings' }
    if (parts.length === 2 && settingSlugToKey(parts[1])) return { page: 'settings', setting: parts[1] as SettingSlug }
    return { page: 'boards' }
  }
  if (parts[0] === 'forum') {
    if (parts.length === 2) return { page: 'boards', forumId: parts[1] }
    if (parts.length === 3) return { page: 'board', forumId: parts[1], slug: parts[2] }
    if (parts.length === 4 && parts[3] === 'new') return { page: 'compose', forumId: parts[1], slug: parts[2] }
    if (parts.length === 4 && /^\d+$/.test(parts[3])) {
      const threadNum = parseInt(parts[3], 10)
      const postNum = hash && /^\d+$/.test(hash) ? parseInt(hash, 10) : undefined
      return { page: 'thread', forumId: parts[1], slug: parts[2], threadNum, postNum }
    }
    return { page: 'boards' }
  }
  if (parts.length === 1 && defaultForumId) {
    return { page: 'board', forumId: defaultForumId, slug: parts[0] }
  }
  if (parts.length === 2 && defaultForumId) {
    if (parts[1] === 'new') return { page: 'compose', forumId: defaultForumId, slug: parts[0] }
    if (/^\d+$/.test(parts[1])) {
      const threadNum = parseInt(parts[1], 10)
      const postNum = hash && /^\d+$/.test(hash) ? parseInt(hash, 10) : undefined
      return { page: 'thread', forumId: defaultForumId, slug: parts[0], threadNum, postNum }
    }
  }
  return { page: 'boards' }
}

export function routeToPath(r: Route): string {
  if (r.page === 'boards') return r.forumId ? boardsPath(r.forumId) : '/'
  if (r.page === 'board') return boardPath(r.forumId, r.slug)
  if (r.page === 'thread') return threadPath(r.forumId, r.slug, r.threadNum, r.postNum)
  if (r.page === 'compose') return composePath(r.forumId, r.slug)
  return r.setting ? `/settings/${r.setting}` : '/settings'
}

export function depthOf(r: Route): number {
  if (r.page === 'boards') return 0
  if (r.page === 'board') return 1
  if (r.page === 'thread') return 2
  if (r.page === 'compose') return 3
  return r.setting ? 2 : 1
}

export function parentRoute(r: Route): Route | null {
  if (r.page === 'thread') return { page: 'board', forumId: r.forumId, slug: r.slug }
  if (r.page === 'compose') return { page: 'board', forumId: r.forumId, slug: r.slug }
  if (r.page === 'board') return { page: 'boards', forumId: r.forumId }
  if (r.page === 'settings' && r.setting) return { page: 'settings' }
  return null
}
