import { createIntentBuilder } from './IntentBuilder'
import { PostEvent } from '../events/post_apply'
import { sendIntents } from '../api/relay'
import { fromHex, computeTweak } from './crypto'
import type { PostObject } from '../bcs/types'

export interface ReactionCtx {
  relayUrl: string
  forumId: string
  nonceShardsId: string
  boardUid: string
}

export async function sendReactionIntent(ctx: ReactionCtx, threadUid: string, post: PostObject, old: Uint8Array | null, next: Uint8Array): Promise<void> {
  const builder = createIntentBuilder({ forumId: ctx.forumId, nonceShardsId: ctx.nonceShardsId, relayUrl: ctx.relayUrl })
    .event(PostEvent.setReactionV2(old, next))
    .board(fromHex(ctx.boardUid))
    .thread(fromHex(threadUid))
    .post(post.root.id)
    .rawTweak(computeTweak(post.root.id))
    .requests({ kind: 'uid' }, { kind: 'ip32', domain: post.root.id })
  const { intentBytes, signature } = await builder.build()
  await sendIntents(ctx.relayUrl, [{ intentBytes, signature }])
}
