import { buildPostRecord, type PostRecord } from '../../store/records'
import { ThreadProjection } from '../../core/bcs/types'
import { fromHex, toHex } from '../../core/intent/crypto'
import { getSecretKeyBytes } from '../../core/keys/storage'
import type { RoleKind } from '../../core/intent/roles'
import type { TooltipPost } from '../../core/load'

export function previewRecord(
  data: TooltipPost,
  roleKinds: RoleKind[],
  pending: { local: string | null; base: string | null } | null,
): PostRecord {
  const uid = toHex(data.post.root.id)
  const opUid = data.threadObj ? toHex(new ThreadProjection(data.threadObj.projection).op()) : null
  const base = data.ctx.myReactions.get(uid) ?? null
  return buildPostRecord({
    post: data.post,
    threadUid: data.threadUid,
    relayUrl: data.ctx.relayUrl,
    forumIdBytes: fromHex(data.ctx.forumId),
    boardUid: data.ctx.boardUid,
    contentMap: data.ctx.contentMap,
    mediaMeta: data.ctx.mediaMeta,
    decryptCtx: data.ctx.decryptCtx,
    moderators: data.ctx.moderators,
    master: getSecretKeyBytes(),
    boardReactions: data.ctx.boardReactions,
    roleKinds,
    op: opUid === uid,
    opAddress: data.opAddress,
    opDeleted: false,
    relNum: 1,
    refs: data.refs ?? [],
    myReaction: pending && pending.base === base ? pending.local : base,
    myReactionBase: base,
    now: Date.now(),
  })
}
