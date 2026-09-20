import { fetchForum } from './api/relay'
import { toHex } from './intent/crypto'

export type RelayMap = Record<string, string>

export const DEFAULT_RELAY_URL = 'https://testnewech.duckdns.org'

const TABS_KEY = 'ech-board-relay-tabs'

export function loadTabs(): RelayMap {
  try {
    const raw = localStorage.getItem(TABS_KEY)
    if (raw) return JSON.parse(raw)
  } catch {}
  return {}
}

export function saveTabs(tabs: RelayMap) {
  localStorage.setItem(TABS_KEY, JSON.stringify(tabs))
}

export function resolveRelayUrl(forumId: string): string | null {
  const tabs = loadTabs()
  const norm = (s: string) => s.replace(/^0x/i, '').toLowerCase()
  const target = forumId ? norm(forumId) : ''
  if (target) {
    for (const key of Object.keys(tabs)) {
      const k = norm(key)
      if (k === target || k.startsWith(target) || target.startsWith(k)) {
        return tabs[key]
      }
    }
  }
  return null
}

export function findForumByShortId(shortId: string): { forumId: string; relayUrl: string } | null {
  const tabs = loadTabs()
  const target = shortId.replace(/^0x/i, '').replace(/^https?:\/\//, '').toLowerCase()
  if (!target) return null
  for (const key of Object.keys(tabs)) {
    const k = key.replace(/^0x/i, '').toLowerCase()
    const url = tabs[key].replace(/^https?:\/\//, '').toLowerCase()
    if (k === target || k.startsWith(target) || target.startsWith(k) || url === target || url.startsWith(target) || target.startsWith(url)) {
      return { forumId: k, relayUrl: tabs[key] }
    }
  }
  return null
}

export async function addRelay(url: string): Promise<string | null> {
  try {
    const forum = await fetchForum(url)
    const forumId = toHex(forum.forum.root.id).replace(/^0x/, '')
    saveTabs({ ...loadTabs(), [forumId]: url })
    return forumId
  } catch {
    return null
  }
}

export async function ensureDefaultRelay(): Promise<string | null> {
  if (Object.keys(loadTabs()).length > 0) return null
  return addRelay(DEFAULT_RELAY_URL)
}

const DEFAULT_FORUM_KEY = 'ech-board-default-forum'

export function getCachedDefaultForumId(): string | null {
  try {
    const raw = localStorage.getItem(DEFAULT_FORUM_KEY)
    return raw || null
  } catch {
    return null
  }
}

export async function ensureDefaultForumId(): Promise<string | null> {
  const cached = getCachedDefaultForumId()
  if (cached && resolveRelayUrl(cached)) return cached
  const forumId = await addRelay(DEFAULT_RELAY_URL)
  if (forumId) {
    try { localStorage.setItem(DEFAULT_FORUM_KEY, forumId) } catch {}
  }
  return forumId
}
