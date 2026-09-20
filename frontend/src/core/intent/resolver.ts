import { getForumInfo } from '../api/cache'
import { fetchResolvePost } from '../api/relay'
import { findForumByShortId } from '../config'
import { fromHex, toHex } from './crypto'
import type { PostResolver } from './bbcode'

export function makePostResolver(ctx: {
  forumId: string
  boardUid?: string
  slug?: string
  relayUrl: string
  postMap?: Map<number, { id: Uint8Array; author: Uint8Array }>
}): PostResolver {
  return async (ref) => {
    let targetForumId = ctx.forumId
    let targetRelay = ctx.relayUrl
    if (ref.forum) {
      const known = findForumByShortId(ref.forum)
      if (!known) return null
      targetForumId = known.forumId
      targetRelay = known.relayUrl
    }
    let targetBoardUid: string
    let targetSlug: string
    if (ref.board) {
      const forum = await getForumInfo(targetRelay)
      const buid = forum.boardBySlug.get(ref.board)
      if (!buid) return null
      targetBoardUid = buid
      targetSlug = ref.board
    } else {
      if (targetForumId !== ctx.forumId || !ctx.boardUid || !ctx.slug) return null
      targetBoardUid = ctx.boardUid
      targetSlug = ctx.slug
    }
    let postUid: string | null = null
    let author: Uint8Array | undefined
    if (targetForumId === ctx.forumId && targetBoardUid === ctx.boardUid) {
      const pinfo = ctx.postMap?.get(ref.postNum)
      if (pinfo) {
        postUid = toHex(pinfo.id)
        author = pinfo.author
      }
    }
    if (!postUid) {
      postUid = await fetchResolvePost(targetRelay, targetBoardUid, ref.postNum)
      if (!postUid) return null
    }
    return {
      forum: fromHex(targetForumId),
      board: fromHex(targetBoardUid),
      post: fromHex(postUid),
      hint: `${targetSlug}/${ref.postNum}`,
      author,
    }
  }
}
