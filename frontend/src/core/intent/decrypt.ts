import { PostProjection, type PostObject } from '../bcs/chain'
import { toHex, derivePostKeypair } from './crypto'

export interface DecryptContext {
  masterSecret: Uint8Array
  forumId: Uint8Array
  postAuthors: Map<string, Uint8Array>
  myAuthorTweaks: Map<string, Uint8Array>
  postNums: Map<string, number>
}

export function buildDecryptContext(posts: PostObject[], masterSecret: Uint8Array, forumIdBytes: Uint8Array): DecryptContext {
  return addPostsToDecryptContext(
    { masterSecret, forumId: forumIdBytes, postAuthors: new Map(), myAuthorTweaks: new Map(), postNums: new Map() },
    posts,
  )
}

export function addPostsToDecryptContext(ctx: DecryptContext, posts: readonly PostObject[]): DecryptContext {
  let postAuthors: Map<string, Uint8Array> | null = null
  let myAuthorTweaks: Map<string, Uint8Array> | null = null
  let postNums: Map<string, number> | null = null
  for (const p of posts) {
    const id = toHex(p.root.id)
    if ((postAuthors ?? ctx.postAuthors).has(id)) continue
    const pp = new PostProjection(p.projection)
    const sender = pp.sender()
    const pk = toHex(sender.pk)
    postAuthors ??= new Map(ctx.postAuthors)
    myAuthorTweaks ??= new Map(ctx.myAuthorTweaks)
    postNums ??= new Map(ctx.postNums)
    postAuthors.set(id, sender.pk)
    postNums.set(id, Number(pp.number()))
    if (!myAuthorTweaks.has(pk)) {
      const derived = derivePostKeypair(ctx.masterSecret, ctx.forumId, sender.tweak)
      if (toHex(derived.publicKey) === pk) myAuthorTweaks.set(pk, sender.tweak)
    }
  }
  if (!postAuthors) return ctx
  return { masterSecret: ctx.masterSecret, forumId: ctx.forumId, postAuthors, myAuthorTweaks: myAuthorTweaks!, postNums: postNums! }
}
