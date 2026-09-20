import { createContext, useContext, type JSX } from 'solid-js'
import { createModActions, type ModActions } from './modActions'
import { useStore } from './store'
import { postUid } from '../core/posts'
import { PostProjection } from '../core/bcs/types'
import { toHex } from '../core/intent/crypto'
import type { RoleOption } from '../core/intent/roles'

const ModCtx = createContext<ModActions>()

export function ModProvider(props: { roles: RoleOption[]; onError: (e: unknown) => void; children: JSX.Element }) {
  const store = useStore()
  const actions = createModActions(() => {
    const info = store.info()!
    const ctx = store.decryptCtx()!
    return {
      forumId: info.forumId,
      nonceShardsId: info.nonceShardsId,
      relayUrl: info.relayUrl,
      boardUid: info.boardUid,
      roles: props.roles,
      myTweaks: ctx.myAuthorTweaks,
      postByUid: (uidHex) => {
        for (const uid of store.postsOf(true)) {
          const record = store.get(uid)
          if (record && toHex(new PostProjection(record.post.projection).uid()) === uidHex) return uid
        }
        return null
      },
      onPostContent: (hashHex, blob) => store.addContent(hashHex, blob),
      onPostDone: (post) => {
        const uid = postUid(post)
        setTimeout(() => void store.refetchPost(uid), 200)
      },
      onError: props.onError,
    }
  })
  return <ModCtx.Provider value={actions}>{props.children}</ModCtx.Provider>
}

export function useMod(): ModActions {
  const actions = useContext(ModCtx)
  if (!actions) throw new Error('useMod outside provider')
  return actions
}
