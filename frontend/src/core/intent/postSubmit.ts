import { blake2b } from '@noble/hashes/blake2'
import { createIntentBuilder } from './IntentBuilder'
import { parseBbCode, finalizeParts } from './bbcode'
import { makePostResolver } from './resolver'
import { fromHex } from './crypto'
import { stripExif } from './exif'
import { BoardEvent as IBoardEvent } from '../events/board_apply'
import { serializePostParts } from '../bcs/types'
import { sendIntents } from '../api/relay'
import type { RoleOption } from './roles'

export interface PostFormState {
  text: string
  name: string
  withFlag: boolean
  files: File[]
  stripExif: boolean
}

export interface PostSubmitCtx {
  forumId: string
  nonceShardsId: string
  relayUrl: string
  boardUid: string
  slug?: string
  role?: RoleOption
}

function parseNameAndTripcode(name: string): { nameText: string; nameHash: Uint8Array | null; tripcodeSuffix: string | undefined } {
  const enc = new TextEncoder()
  const hashIdx = name.indexOf('#')
  const nameText = (hashIdx >= 0 ? name.slice(0, hashIdx) : name).trim()
  const nameHash = nameText ? blake2b(enc.encode(nameText), { dkLen: 32 }) : null
  const tripcodeSuffix = hashIdx >= 0 ? name.slice(hashIdx) : undefined
  return { nameText, nameHash, tripcodeSuffix }
}

async function packMedia(form: PostFormState): Promise<{ mediaBlobs: Uint8Array[]; mediaHashes: Uint8Array[] }> {
  const mediaBlobs: Uint8Array[] = []
  const mediaHashes: Uint8Array[] = []
  for (const f of form.files) {
    const buf = await f.arrayBuffer()
    const bytes = form.stripExif && f.type.startsWith('image/')
      ? await stripExif(new Uint8Array(buf))
      : new Uint8Array(buf)
    mediaBlobs.push(bytes)
    mediaHashes.push(blake2b(bytes, { dkLen: 32 }))
  }
  return { mediaBlobs, mediaHashes }
}

async function buildTextBlob(
  ctx: PostSubmitCtx,
  text: string,
  secretKey: Uint8Array,
  publicKey: Uint8Array,
  postMap?: Map<number, { id: Uint8Array; author: Uint8Array }>,
): Promise<{ textBlob: Uint8Array | null; textHash: Uint8Array | null }> {
  const resolver = makePostResolver({ forumId: ctx.forumId, boardUid: ctx.boardUid, slug: ctx.slug, relayUrl: ctx.relayUrl, postMap })
  const parsed = await parseBbCode(text.trim(), resolver)
  const parts = await finalizeParts(parsed, secretKey, publicKey)
  const textBlob = parts.length > 0 ? serializePostParts(parts) : null
  const textHash = textBlob ? blake2b(textBlob, { dkLen: 32 }) : null
  return { textBlob, textHash }
}

export async function submitNewThread(ctx: PostSubmitCtx, form: PostFormState, subject: string, onProgress?: (loaded: number, total: number) => void, signal?: AbortSignal) {
  let builder = createIntentBuilder(ctx).role(ctx.role)
  const { secretKey, publicKey } = builder.deriveKeypair()

  const { textBlob, textHash } = await buildTextBlob(ctx, form.text, secretKey, publicKey)
  const { mediaBlobs, mediaHashes } = await packMedia(form)
  const topicText = subject.trim()
  const topicHash = topicText ? blake2b(new TextEncoder().encode(topicText), { dkLen: 32 }) : null
  const { nameText, nameHash, tripcodeSuffix } = parseNameAndTripcode(form.name)

  builder = builder
    .event(IBoardEvent.newThreadV2(topicHash, textHash, mediaHashes, nameHash, [], false))
    .board(fromHex(ctx.boardUid))
    .captcha()
  if (tripcodeSuffix) builder = builder.tripcode()
  if (form.withFlag) builder = builder.geo()
  const { intentBytes, signature } = await builder.build()

  return sendIntents(ctx.relayUrl, [{
    intentBytes,
    signature,
    text: textBlob ?? undefined,
    media: mediaBlobs.length ? mediaBlobs : undefined,
    topic: topicText || undefined,
    name: nameText || undefined,
    tripcode: tripcodeSuffix,
  }], { onProgress, signal })
}

export async function submitReply(
  ctx: PostSubmitCtx & { threadUid: string; postMap?: Map<number, { id: Uint8Array; author: Uint8Array }> },
  form: PostFormState,
  onProgress?: (loaded: number, total: number) => void,
  signal?: AbortSignal,
) {
  let builder = createIntentBuilder(ctx).role(ctx.role)
  const { secretKey, publicKey } = builder.deriveKeypair()

  const { textBlob, textHash } = await buildTextBlob(ctx, form.text, secretKey, publicKey, ctx.postMap)
  const { mediaBlobs, mediaHashes } = await packMedia(form)
  const { nameText, nameHash, tripcodeSuffix } = parseNameAndTripcode(form.name)

  builder = builder
    .event(IBoardEvent.newPostV2(fromHex(ctx.threadUid), textHash, mediaHashes, nameHash, [], false))
    .board(fromHex(ctx.boardUid))
    .thread(fromHex(ctx.threadUid))
    .captcha()
  if (tripcodeSuffix) builder = builder.tripcode()
  if (form.withFlag) builder = builder.geo()
  const { intentBytes, signature } = await builder.build()

  return sendIntents(ctx.relayUrl, [{
    intentBytes,
    signature,
    text: textBlob ?? undefined,
    media: mediaBlobs.length ? mediaBlobs : undefined,
    name: nameText || undefined,
    tripcode: tripcodeSuffix,
  }], { onProgress, signal })
}
