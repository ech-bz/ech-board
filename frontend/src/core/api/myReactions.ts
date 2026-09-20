import { fetchReactions } from './relay'
import { getSecretKeyBytes } from '../keys/storage'
import { fromHex, derivePostKeypair, computeTweak } from '../intent/crypto'
import { PostProjection, type PostObject } from '../bcs/types'

export async function fetchMyReactions(relayUrl: string, posts: PostObject[], forumId: string): Promise<Map<string, string>> {
  const masterSecret = getSecretKeyBytes()
  const forumIdBytes = fromHex(forumId)
  const pairs: [Uint8Array, Uint8Array][] = []
  for (const p of posts) {
    const pp = new PostProjection(p.projection)
    if (pp.reactions().length > 0) {
      pairs.push([p.root.id, derivePostKeypair(masterSecret, forumIdBytes, computeTweak(p.root.id)).publicKey])
    }
  }
  const my = new Map<string, string>()
  if (pairs.length === 0) return my
  try {
    for (const [pid, r] of await fetchReactions(relayUrl, pairs)) {
      my.set(pid, r)
    }
  } catch {}
  return my
}
