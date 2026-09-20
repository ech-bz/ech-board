import { createSignal } from 'solid-js'
import { blake2b } from '@noble/hashes/blake2'
import { createIntentBuilder } from '../core/intent/IntentBuilder'
import { PostEvent } from '../core/events/post_apply'
import { ThreadEvent } from '../core/events/thread_apply'
import { ForumEvent } from '../core/events/forum_apply'
import { BoardEvent } from '../core/events/board_apply'
import { PostProjection, serializePostParts } from '../core/bcs/types'
import type { PostObject, ThreadObject, BoardObject, PostPartInput } from '../core/bcs/types'
import type { EventDesc } from '../core/events/core'
import {
  roleForPostAction,
  roleForThreadAction,
  roleForBan,
  roleForForumSettings,
  roleForBoardCore,
  roleForBoardSoft,
  roleForThreadAdminMgmt,
  SELF_MOD_WINDOW_MS,
} from '../core/intent/capabilities'
import type { BanScope } from '../core/intent/capabilities'
import { fromHex, toHex, computeTweak, derivePostKeypair, sign } from '../core/intent/crypto'
import { parseBbCode, finalizeParts } from '../core/intent/bbcode'
import { makePostResolver } from '../core/intent/resolver'
import { matchPostsByAuthor } from '../core/intent/author'
import { sendIntents, fetchBanHashes, fetchNonce } from '../core/api/relay'
import { getSecretKeyBytes } from '../core/keys/storage'
import type { RoleOption } from '../core/intent/roles'
import type { EntityNs } from '../core/intent/upgrade'

export interface ModEnv {
  forumId: string
  nonceShardsId: string
  relayUrl: string
  boardUid: string
  slug?: string
  roles: RoleOption[]
  myTweaks: Map<string, Uint8Array>
  postByUid: (uidHex: string) => string | null
  onDone?: () => Promise<void> | void
  onPostDone?: (post: PostObject) => Promise<void> | void
  onPostContent?: (hashHex: string, blob: Uint8Array) => void
  onError: (e: unknown) => void
}

type PostModAction = 'delete' | 'restore'
type ThreadModAction = 'close' | 'open' | 'delete' | 'restore'

const BATCH_CHUNK = 100

function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((n, a) => n + a.length, 0)
  const out = new Uint8Array(total)
  let off = 0
  for (const a of arrays) {
    out.set(a, off)
    off += a.length
  }
  return out
}

function banScopeVars(scope: BanScope, forumId: string, boardUid: string, threadUid: string, postUid: string | null, durationMs: number, reason: string) {
  if (scope === 'thread' && postUid === null) throw new Error('thread ban requires the post address')
  const pathHex = scope === 'forum'
    ? [forumId]
    : scope === 'board'
      ? [forumId, boardUid]
      : [forumId, boardUid, threadUid, postUid!]
  const reasonHash = blake2b(new TextEncoder().encode(reason), { dkLen: 32 })
  const expires = Date.now() + durationMs
  const level = fromHex(scope === 'forum' ? forumId : scope === 'board' ? boardUid : threadUid)
  return { pathHex, reasonHash, expires, level }
}

async function buildBanEvent(
  uid: Uint8Array,
  scope: BanScope,
  mask: number,
  vars: { pathHex: string[]; reasonHash: Uint8Array; expires: number; level: Uint8Array },
  secretKey: Uint8Array,
  publicKey: Uint8Array,
  relayUrl: string,
): Promise<EventDesc> {
  const msg = concatBytes(uid, ...vars.pathHex.map(fromHex), publicKey)
  const signature = await sign(secretKey, msg)
  const hashes = await fetchBanHashes(relayUrl, uid, vars.pathHex, publicKey, signature)
  const maskIndex = mask === 32 ? 0 : mask === 24 ? 1 : mask === 20 ? 2 : 3
  const ipHash = hashes[maskIndex].slice().reverse()
  return scope === 'forum'
    ? ForumEvent.banUser(vars.level, mask, ipHash, vars.reasonHash, vars.expires)
    : scope === 'board'
      ? BoardEvent.banUser(vars.level, mask, ipHash, vars.reasonHash, vars.expires)
      : ThreadEvent.banUser(vars.level, mask, ipHash, vars.reasonHash, vars.expires)
}

export function createModActions(getEnv: () => ModEnv) {
  const [busy, setBusy] = createSignal(false)

  const runAction = async (fn: () => Promise<void>, done?: () => Promise<void> | void) => {
    if (busy()) return
    const env = getEnv()
    setBusy(true)
    try {
      await fn()
      await (done ?? env.onDone)?.()
    } catch (e) {
      env.onError(e)
    }
    setBusy(false)
  }

  const signPost = (post: PostObject, env: ModEnv) => {
    const pp = new PostProjection(post.projection)
    const myTweak = env.myTweaks.get(toHex(pp.sender().pk))
    const ageMs = Date.now() - Number(pp.timestamp_ms())
    if (myTweak && ageMs <= SELF_MOD_WINDOW_MS) return { rawTweak: myTweak } as RoleOption
    const role = roleForPostAction('edit', env.roles)
    if (!role || !role.tweakArgs) throw new Error('no role for post action')
    return role
  }

  const runPostDelete = (post: PostObject, action: PostModAction, threadUid: string) =>
    runAction(async () => {
      const env = getEnv()
      const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
        .event(ThreadEvent.postSetDeleted(action === 'delete'))
        .board(fromHex(env.boardUid))
        .thread(fromHex(threadUid))
        .post(post.root.id)
        .role(signPost(post, env))
      const { intentBytes, signature } = await builder.build()
      await sendIntents(env.relayUrl, [{ intentBytes, signature }])
    })

  const runPostEdit = (
    post: PostObject,
    threadUid: string,
    text: string,
    postMap: Map<number, { id: Uint8Array; author: Uint8Array }>,
    secretRefs?: Map<number, PostPartInput>,
  ) =>
    runAction(
      async () => {
        const env = getEnv()
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(post.root.id)
          .role(signPost(post, env))
        const { secretKey, publicKey } = builder.deriveKeypair()
        const parsed = await parseBbCode(text, makePostResolver({ forumId: env.forumId, boardUid: env.boardUid, slug: env.slug, relayUrl: env.relayUrl, postMap }))
        const parts = await finalizeParts(parsed, secretKey, publicKey, secretRefs)
        const textBlob = parts.length > 0 ? serializePostParts(parts) : null
        const textHash = textBlob ? blake2b(textBlob, { dkLen: 32 }) : null
        const { intentBytes, signature } = await builder.event(ThreadEvent.postSetText(textHash)).build()
        await sendIntents(env.relayUrl, [{ intentBytes, signature, text: textBlob ?? undefined }])
        if (textBlob && textHash) env.onPostContent?.(toHex(textHash), textBlob)
      },
      () => getEnv().onPostDone?.(post),
    )

  const runPostBanMedia = (post: PostObject, threadUid: string, hashes: Uint8Array[]) =>
    runAction(
      async () => {
        const env = getEnv()
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(PostEvent.banMedia(hashes.map((h) => new Uint8Array(h))))
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(post.root.id)
          .role(signPost(post, env))
        const { intentBytes, signature } = await builder.build()
        await sendIntents(env.relayUrl, [{ intentBytes, signature }])
      },
      () => getEnv().onPostDone?.(post),
    )

  const runPostUnbanMedia = (post: PostObject, threadUid: string, hashes: Uint8Array[]) =>
    runAction(
      async () => {
        const env = getEnv()
        const role = roleForPostAction('unban_media', env.roles)
        if (!role || !role.tweakArgs) throw new Error('no role for unban media')
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(PostEvent.unbanMedia(hashes.map((h) => new Uint8Array(h))))
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(post.root.id)
          .role(role)
        const { intentBytes, signature } = await builder.build()
        await sendIntents(env.relayUrl, [{ intentBytes, signature }])
      },
      () => getEnv().onPostDone?.(post),
    )

  const banPost = async (post: PostObject, threadUid: string, scope: BanScope, mask: number, durationMs: number, reason: string) => {
    const env = getEnv()
    const role = roleForBan(scope, env.roles)
    if (!role || !role.tweakArgs) throw new Error(`no role for ${scope} ban`)
    const masterSecret = getSecretKeyBytes()
    const forumIdBytes = fromHex(env.forumId)
    const tweak = computeTweak(...role.tweakArgs)
    const { secretKey, publicKey } = derivePostKeypair(masterSecret, forumIdBytes, tweak)
    const vars = banScopeVars(scope, env.forumId, env.boardUid, threadUid, toHex(post.root.id), durationMs, reason)
    const event = await buildBanEvent(new PostProjection(post.projection).uid(), scope, mask, vars, secretKey, publicKey, env.relayUrl)
    const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl, masterSecret })
      .event(event)
      .board(fromHex(env.boardUid))
      .thread(fromHex(threadUid))
      .post(post.root.id)
      .role(role)
    const { intentBytes, signature } = await builder.build()
    await sendIntents(env.relayUrl, [{ intentBytes, signature }], { reason })
  }

  const banUid = async (uidHex: string, threadUid: string, scope: BanScope, mask: number, durationMs: number, reason: string) => {
    const env = getEnv()
    const role = roleForBan(scope, env.roles)
    if (!role || !role.tweakArgs) throw new Error(`no role for ${scope} ban`)
    const masterSecret = getSecretKeyBytes()
    const forumIdBytes = fromHex(env.forumId)
    const tweak = computeTweak(...role.tweakArgs)
    const { secretKey, publicKey } = derivePostKeypair(masterSecret, forumIdBytes, tweak)
    const postUid = scope === 'thread' ? env.postByUid(uidHex) : null
    if (scope === 'thread' && postUid === null) throw new Error('uid not found in this thread')
    const vars = banScopeVars(scope, env.forumId, env.boardUid, threadUid, postUid, durationMs, reason)
    const event = await buildBanEvent(fromHex(uidHex), scope, mask, vars, secretKey, publicKey, env.relayUrl)
    let builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl, masterSecret })
      .event(event)
      .board(fromHex(env.boardUid))
    if (scope === 'thread') builder = builder.thread(fromHex(threadUid))
    const { intentBytes, signature } = await builder.role(role).build()
    await sendIntents(env.relayUrl, [{ intentBytes, signature }], { reason })
  }

  const runBanUid = (uidHex: string, threadUid: string, scope: BanScope, mask: number, durationMs: number, reason: string) =>
    runAction(() => banUid(uidHex, threadUid, scope, mask, durationMs, reason))

  const runBan = (post: PostObject, threadUid: string, scope: BanScope, mask: number, durationMs: number, reason: string) =>
    runAction(
      () => banPost(post, threadUid, scope, mask, durationMs, reason),
      () => getEnv().onPostDone?.(post),
    )

  const runBatchPostDelete = async (posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void) => {
    const env = getEnv()
    const targets = posts.filter((p) => !new PostProjection(p.projection).deleted())
    const role = roleForPostAction('delete', env.roles)
    if (!role || !role.tweakArgs) throw new Error('no role for batch delete')
    const sender = toHex(
      createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
        .board(fromHex(env.boardUid))
        .thread(fromHex(threadUid))
        .role(role)
        .deriveKeypair().publicKey,
    ).replace(/^0x/, '')
    let done = 0
    for (let start = 0; start < targets.length; start += BATCH_CHUNK) {
      const chunk = targets.slice(start, start + BATCH_CHUNK)
      const base = await fetchNonce(env.relayUrl, sender)
      const items: { intentBytes: Uint8Array; signature: Uint8Array }[] = []
      for (let i = 0; i < chunk.length; i++) {
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(ThreadEvent.postSetDeleted(true))
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(chunk[i].root.id)
          .role(role)
        const { intentBytes, signature } = await builder.build(base + i)
        items.push({ intentBytes, signature })
        done++
        onProgress?.(done, targets.length)
      }
      await sendIntents(env.relayUrl, items)
    }
  }

  const runBatchPostRestore = async (posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void) => {
    const env = getEnv()
    const targets = posts.filter((p) => new PostProjection(p.projection).deleted())
    const role = roleForPostAction('restore', env.roles)
    if (!role || !role.tweakArgs) throw new Error('no role for batch restore')
    const sender = toHex(
      createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
        .board(fromHex(env.boardUid))
        .thread(fromHex(threadUid))
        .role(role)
        .deriveKeypair().publicKey,
    ).replace(/^0x/, '')
    let done = 0
    for (let start = 0; start < targets.length; start += BATCH_CHUNK) {
      const chunk = targets.slice(start, start + BATCH_CHUNK)
      const base = await fetchNonce(env.relayUrl, sender)
      const items: { intentBytes: Uint8Array; signature: Uint8Array }[] = []
      for (let i = 0; i < chunk.length; i++) {
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(ThreadEvent.postSetDeleted(false))
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(chunk[i].root.id)
          .role(role)
        const { intentBytes, signature } = await builder.build(base + i)
        items.push({ intentBytes, signature })
        done++
        onProgress?.(done, targets.length)
      }
      await sendIntents(env.relayUrl, items)
    }
  }

  const runBatchBan = async (
    posts: PostObject[],
    threadUid: string,
    scope: BanScope,
    mask: number,
    durationMs: number,
    reason: string,
    onProgress?: (done: number, total: number) => void,
  ) => {
    const env = getEnv()
    const role = roleForBan(scope, env.roles)
    if (!role || !role.tweakArgs) throw new Error(`no role for ${scope} ban`)
    const masterSecret = getSecretKeyBytes()
    const forumIdBytes = fromHex(env.forumId)
    const tweak = computeTweak(...role.tweakArgs)
    const { secretKey, publicKey } = derivePostKeypair(masterSecret, forumIdBytes, tweak)
    const sender = toHex(publicKey).replace(/^0x/, '')
    let done = 0
    for (let start = 0; start < posts.length; start += BATCH_CHUNK) {
      const chunk = posts.slice(start, start + BATCH_CHUNK)
      const base = await fetchNonce(env.relayUrl, sender)
      const items: { intentBytes: Uint8Array; signature: Uint8Array }[] = []
      for (let i = 0; i < chunk.length; i++) {
        const post = chunk[i]
        const vars = banScopeVars(scope, env.forumId, env.boardUid, threadUid, toHex(post.root.id), durationMs, reason)
        const event = await buildBanEvent(new PostProjection(post.projection).uid(), scope, mask, vars, secretKey, publicKey, env.relayUrl)
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(event)
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(post.root.id)
          .role(role)
        const { intentBytes, signature } = await builder.build(base + i)
        items.push({ intentBytes, signature })
        done++
        onProgress?.(done, posts.length)
      }
      await sendIntents(env.relayUrl, items, { reason })
    }
  }

  const runUnban = (post: PostObject, threadUid: string, scope: BanScope) =>
    runAction(
      async () => {
        const env = getEnv()
        const banned = new PostProjection(post.projection).banned()
        if (!banned) throw new Error('post is not banned')
        const role = roleForBan(scope, env.roles)
        if (!role || !role.tweakArgs) throw new Error(`no role for ${scope} unban`)
        const event =
          scope === 'forum'
            ? ForumEvent.unbanUser(banned.level, banned.mask, banned.ip_hash)
            : scope === 'board'
              ? BoardEvent.unbanUser(banned.level, banned.mask, banned.ip_hash)
              : ThreadEvent.unbanUser(banned.level, banned.mask, banned.ip_hash)
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(event)
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .post(post.root.id)
          .role(role)
        const { intentBytes, signature } = await builder.build()
        await sendIntents(env.relayUrl, [{ intentBytes, signature }])
      },
      () => getEnv().onPostDone?.(post),
    )

  const sendThreadEvent = async (action: ThreadModAction, threadUid: string) => {
    const env = getEnv()
    const desc =
      action === 'close'
        ? ThreadEvent.setClosed(true)
        : action === 'open'
          ? ThreadEvent.setClosed(false)
          : action === 'delete'
            ? ThreadEvent.setDeleted(true)
            : ThreadEvent.setDeleted(false)
    const role = roleForThreadAction(action, env.roles)
    if (!role || !role.tweakArgs) throw new Error(`no role for ${action}`)
    const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
      .event(desc)
      .board(fromHex(env.boardUid))
      .thread(fromHex(threadUid))
      .role(role)
    const { intentBytes, signature } = await builder.build()
    await sendIntents(env.relayUrl, [{ intentBytes, signature }])
  }

  const runThreadAction = (action: ThreadModAction, threadUid: string) => runAction(() => sendThreadEvent(action, threadUid))

  const runThreadCloseDelete = (threadUid: string) =>
    runAction(async () => {
      const env = getEnv()
      const role = roleForThreadAction('delete', env.roles)
      if (!role || !role.tweakArgs) throw new Error('no role for close/delete')
      const sender = toHex(
        createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .role(role)
          .deriveKeypair().publicKey,
      ).replace(/^0x/, '')
      const base = await fetchNonce(env.relayUrl, sender)
      const items: { intentBytes: Uint8Array; signature: Uint8Array }[] = []
      for (const [i, action] of (['close', 'delete'] as ThreadModAction[]).entries()) {
        const desc = action === 'close' ? ThreadEvent.setClosed(true) : ThreadEvent.setDeleted(true)
        const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(desc)
          .board(fromHex(env.boardUid))
          .thread(fromHex(threadUid))
          .role(role)
        const { intentBytes, signature } = await builder.build(base + i)
        items.push({ intentBytes, signature })
      }
      await sendIntents(env.relayUrl, items)
    })

  const runThreadEditTopic = (threadUid: string, topic: string) =>
    runAction(async () => {
      const env = getEnv()
      const topicText = topic.trim()
      const topicHash = topicText ? blake2b(new TextEncoder().encode(topicText), { dkLen: 32 }) : null
      const role = roleForThreadAction('edit_topic', env.roles)
      if (!role || !role.tweakArgs) throw new Error('no role for edit_topic')
      const builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
        .event(ThreadEvent.setTopic(topicHash))
        .board(fromHex(env.boardUid))
        .thread(fromHex(threadUid))
        .role(role)
      const { intentBytes, signature } = await builder.build()
      await sendIntents(env.relayUrl, [{ intentBytes, signature, topic: topicText || undefined }])
      if (topicText && topicHash) env.onPostContent?.(toHex(topicHash), new TextEncoder().encode(topicText))
    })

  const runUpgrade = (ns: EntityNs, object: PostObject | ThreadObject | BoardObject | null) =>
    runAction(
      async () => {
        if (ns === 'forum' || !object) return
        const env = getEnv()
        const desc = ns === 'board' ? BoardEvent.upgrade() : ns === 'thread' ? ThreadEvent.upgrade() : PostEvent.upgrade()
        const threadUid =
          ns === 'thread' || ns === 'post'
            ? toHex(ns === 'post' ? new PostProjection((object as PostObject).projection).thread() : object.root.id)
            : null
        let builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
          .event(desc)
          .board(fromHex(env.boardUid))
        if (threadUid) builder = builder.thread(fromHex(threadUid))
        if (ns === 'post') builder = builder.post(object.root.id)
        const { intentBytes, signature } = await builder.build()
        await sendIntents(env.relayUrl, [{ intentBytes, signature }])
      },
      () => {
        if (ns !== 'post' || !object) return
        getEnv().onPostDone?.(object as PostObject)
      },
    )

  const runIntent = (
    desc: EventDesc,
    role: RoleOption | null,
    threadUid: string | null,
    extras?: { textBlob?: Uint8Array; descriptionText?: string; topicText?: string },
    boardUidArg?: string,
  ) =>
    runAction(async () => {
      const env = getEnv()
      if (!role || !role.tweakArgs) throw new Error('no role for action')
      let builder = createIntentBuilder({ forumId: env.forumId, nonceShardsId: env.nonceShardsId, relayUrl: env.relayUrl })
        .event(desc)
        .board(fromHex(boardUidArg ?? env.boardUid))
      if (threadUid) builder = builder.thread(fromHex(threadUid))
      builder = builder.role(role)
      const { intentBytes, signature } = await builder.build()
      await sendIntents(env.relayUrl, [{ intentBytes, signature, text: extras?.textBlob, description: extras?.descriptionText, topic: extras?.topicText }])
    })

  const runForumSetTimestampPrecision = (precision: number) =>
    runIntent(ForumEvent.setTimestampPrecision(precision), roleForForumSettings(getEnv().roles), null)

  const runBoardSetDescription = (text: string) => {
    const hash = text ? blake2b(new TextEncoder().encode(text), { dkLen: 32 }) : null
    return runIntent(BoardEvent.setDescription(hash), roleForBoardSoft(getEnv().roles), null, { descriptionText: text })
  }

  const runBoardSetMaxMedia = (v: number) => runIntent(BoardEvent.setMaxMedia(v), roleForBoardCore(getEnv().roles), null)

  const runBoardSetBumpLimit = (v: number) => runIntent(BoardEvent.setBumpLimit(v), roleForBoardCore(getEnv().roles), null)

  const runBoardSetIgnoreForumBans = (v: boolean) => runIntent(BoardEvent.setIgnoreForumBans(v), roleForBoardCore(getEnv().roles), null)

  const runBoardSetReactions = (vals: Uint8Array[]) => runIntent(BoardEvent.setReactions(vals), roleForBoardSoft(getEnv().roles), null)

  const runBoardSetPinned = (pinned: Uint8Array[], boardUidArg?: string) =>
    runIntent(BoardEvent.setPinned(pinned), roleForBoardSoft(getEnv().roles), null, undefined, boardUidArg)

  const runBoardAddModerator = (addr: Uint8Array) => runIntent(BoardEvent.addModerator(addr), roleForBoardCore(getEnv().roles), null)

  const runBoardDelModerator = (addr: Uint8Array) => runIntent(BoardEvent.delModerator(addr), roleForBoardCore(getEnv().roles), null)

  const runBoardClose = (boardUidArg: string, v: boolean) => runIntent(BoardEvent.setClosed(v), roleForBoardCore(getEnv().roles), null, undefined, boardUidArg)

  const runBoardDelete = (boardUidArg: string, v: boolean) => runIntent(BoardEvent.setDeleted(v), roleForBoardCore(getEnv().roles), null, undefined, boardUidArg)

  const runThreadSetAdmin = (threadUid: string, addr: Uint8Array | null) =>
    runIntent(ThreadEvent.setAdmin(addr), roleForThreadAdminMgmt(getEnv().roles), threadUid)

  const runThreadAddModerator = (threadUid: string, addr: Uint8Array) =>
    runIntent(ThreadEvent.addModerator(addr), roleForThreadAdminMgmt(getEnv().roles), threadUid)

  const runThreadDelModerator = (threadUid: string, addr: Uint8Array) =>
    runIntent(ThreadEvent.delModerator(addr), roleForThreadAdminMgmt(getEnv().roles), threadUid)

  const findAuthorPosts = (ref: PostObject, posts: PostObject[], threadUid: string, onProgress?: (done: number, total: number) => void): Promise<string[]> => {
    const env = getEnv()
    const role = roleForBan('thread', env.roles)
    if (!role || !role.tweakArgs) throw new Error('no role for author match')
    const { secretKey, publicKey } = derivePostKeypair(getSecretKeyBytes(), fromHex(env.forumId), computeTweak(...role.tweakArgs))
    return matchPostsByAuthor({
      relayUrl: env.relayUrl,
      pathHex: [env.forumId, env.boardUid, threadUid],
      pk: publicKey,
      secretKey,
      refId: toHex(ref.root.id),
      candidates: posts.map((p) => ({ id: toHex(p.root.id), uid: new PostProjection(p.projection).uid() })),
      onProgress,
    })
  }

  return {
    busy,
    runPostDelete,
    runPostEdit,
    runPostBanMedia,
    runPostUnbanMedia,
    runThreadAction,
    runThreadCloseDelete,
    runThreadEditTopic,
    runUpgrade,
    runBan,
    runBanUid,
    runUnban,
    runBatchPostDelete,
    runBatchPostRestore,
    runBatchBan,
    runForumSetTimestampPrecision,
    runBoardSetDescription,
    runBoardSetMaxMedia,
    runBoardSetBumpLimit,
    runBoardSetIgnoreForumBans,
    runBoardSetReactions,
    runBoardSetPinned,
    runBoardAddModerator,
    runBoardDelModerator,
    runBoardClose,
    runBoardDelete,
    runThreadSetAdmin,
    runThreadAddModerator,
    runThreadDelModerator,
    findAuthorPosts,
  }
}

export type ModActions = ReturnType<typeof createModActions>
