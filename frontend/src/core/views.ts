import { loadTabs } from './config'
import { getForumInfo } from './api/cache'

export interface ForumInfo {
  forumId: string
  relayUrl: string
  host: string
}

export interface BoardInfo {
  slug: string
  uid: string
  title: string
  count: number
}

export function listForums(): ForumInfo[] {
  const tabs = loadTabs()
  const out: ForumInfo[] = []
  for (const [forumId, url] of Object.entries(tabs)) {
    let host = url
    try {
      host = new URL(url).host
    } catch {
      host = url
    }
    out.push({ forumId, relayUrl: url, host })
  }
  return out
}

export async function loadBoards(relayUrl: string): Promise<{ forumId: string; boards: BoardInfo[] }> {
  const forum = await getForumInfo(relayUrl)
  const boards: BoardInfo[] = []
  for (const [slug, uid] of forum.boardBySlug) {
    if (forum.boardDeleted.has(slug)) continue
    boards.push({ slug, uid, title: forum.boardDescriptions.get(slug)!, count: forum.boardPostCounts.get(slug)! })
  }
  boards.sort((a, b) => b.count - a.count)
  return { forumId: forum.forumId.replace(/^0x/, ''), boards }
}

