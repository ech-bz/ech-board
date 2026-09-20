import { BcsWriter } from '@mysten/bcs'
import { blake2b } from '@noble/hashes/blake2'
import type { EventDesc } from '../events/core'
import { sign, toHex, derivePostKeypair, senderAddress, computeTweak } from './crypto'
import { deriveShardId } from './derive'
import { fetchNonce } from '../api/relay'
import { fromHex } from './crypto'
import { getSecretKeyBytes } from '../keys/storage'
import type { RoleOption } from './roles'

const CLOCK = fromHex('0x0000000000000000000000000000000000000000000000000000000000000006')

interface IntentObject {
  id: Uint8Array
  mutable: boolean
}

interface IntentRequest {
  kind: 'uid' | 'ip32' | 'tripcode' | 'geo' | 'captcha'
  domain?: Uint8Array
}

interface IntentData {
  module: Uint8Array
  function: Uint8Array
  nonce: number
  objects: IntentObject[]
  requests: IntentRequest[]
  payload: Uint8Array
  publicKey: Uint8Array
  tweak: Uint8Array
}

const MODULE = new TextEncoder().encode('main')

function resolveFunction(desc: EventDesc, tripcode: boolean, geo: boolean, captcha: boolean, hasPost: boolean): string {
  if (desc.name === 'new_post' || desc.name === 'new_post_v2' || desc.name === 'new_post_migrate_v2') {
    if (captcha && desc.name === 'new_post_v2') {
      if (tripcode && geo) return 'board_apply_thread_intent_uid_geo_tripcode_captcha'
      if (tripcode) return 'board_apply_thread_intent_uid_tripcode_captcha'
      if (geo) return 'board_apply_thread_intent_uid_geo_captcha'
      return 'board_apply_thread_intent_uid_captcha'
    }
    if (tripcode && geo) return 'board_apply_thread_intent_uid_geo_tripcode'
    if (tripcode) return 'board_apply_thread_intent_uid_tripcode'
    if (geo) return 'board_apply_thread_intent_uid_geo'
    return 'board_apply_thread_intent_uid'
  }
  const postAction = desc.name === 'ban' || desc.name === 'unban'
    || desc.name === 'post_set_deleted' || desc.name === 'post_set_text'
  if (desc._ns === 'forum') {
    return postAction && hasPost ? 'forum_apply_post_intent_uid' : 'forum_apply_intent_uid'
  }
  if (desc._ns === 'board') {
    if (desc.name === 'new_thread' || desc.name === 'new_thread_v2' || desc.name === 'new_thread_v3' || desc.name === 'new_thread_migrate_v2') {
      if (captcha && (desc.name === 'new_thread_v2' || desc.name === 'new_thread_v3')) {
        if (tripcode && geo) return 'board_apply_intent_uid_geo_tripcode_captcha'
        if (tripcode) return 'board_apply_intent_uid_tripcode_captcha'
        if (geo) return 'board_apply_intent_uid_geo_captcha'
        return 'board_apply_intent_uid_captcha'
      }
      if (tripcode && geo) return 'board_apply_intent_uid_geo_tripcode'
      if (tripcode) return 'board_apply_intent_uid_tripcode'
      if (geo) return 'board_apply_intent_uid_geo'
    }
    return postAction && hasPost ? 'board_apply_post_intent_uid' : 'board_apply_intent_uid'
  }
  if (desc._ns === 'thread') {
    return postAction && hasPost ? 'thread_apply_post_intent_uid' : 'thread_apply_intent_uid'
  }
  if (desc._ns === 'post') {
    if (desc.name === 'set_reaction_v2' || desc.name === 'vote_v2') return 'post_apply_intent_uid_ip32'
    return 'post_apply_intent_uid'
  }
  throw new Error(`unknown namespace: ${desc._ns}`)
}

function resolveObjects(
  desc: EventDesc,
  forumId: Uint8Array,
  boardId?: Uint8Array,
  threadId?: Uint8Array,
  postId?: Uint8Array,
  tripcode: boolean = false,
  geo: boolean = false,
  captcha: boolean = false,
): IntentObject[] {
  const fn = resolveFunction(desc, tripcode, geo, captcha, !!postId)
  switch (fn) {
    case 'forum_apply_intent_uid':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: true },
      ]
    case 'forum_apply_post_intent_uid':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: true },
        { id: boardId!, mutable: false },
        { id: threadId!, mutable: false },
        { id: postId!, mutable: true },
      ]
    case 'board_apply_intent_uid':
    case 'board_apply_intent_uid_tripcode':
    case 'board_apply_intent_uid_geo':
    case 'board_apply_intent_uid_geo_tripcode':
    case 'board_apply_intent_uid_captcha':
    case 'board_apply_intent_uid_tripcode_captcha':
    case 'board_apply_intent_uid_geo_captcha':
    case 'board_apply_intent_uid_geo_tripcode_captcha':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: true },
      ]
    case 'board_apply_thread_intent_uid':
    case 'board_apply_thread_intent_uid_tripcode':
    case 'board_apply_thread_intent_uid_geo':
    case 'board_apply_thread_intent_uid_geo_tripcode':
    case 'board_apply_thread_intent_uid_captcha':
    case 'board_apply_thread_intent_uid_tripcode_captcha':
    case 'board_apply_thread_intent_uid_geo_captcha':
    case 'board_apply_thread_intent_uid_geo_tripcode_captcha':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: true },
        { id: threadId!, mutable: true },
      ]
    case 'board_apply_post_intent_uid':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: true },
        { id: threadId!, mutable: false },
        { id: postId!, mutable: true },
      ]
    case 'thread_apply_intent_uid':
      return [
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: false },
        { id: threadId!, mutable: true },
      ]
    case 'thread_apply_post_intent_uid':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: false },
        { id: threadId!, mutable: true },
        { id: postId!, mutable: true },
      ]
    case 'post_apply_intent_uid':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: false },
        { id: threadId!, mutable: false },
        { id: postId!, mutable: true },
      ]
    case 'post_apply_intent_uid_ip32':
      return [
        { id: CLOCK, mutable: false },
        { id: Uint8Array.from([]), mutable: true },
        { id: forumId, mutable: false },
        { id: boardId!, mutable: false },
        { id: threadId!, mutable: false },
        { id: postId!, mutable: true },
      ]
    default:
      throw new Error(`unknown function: ${fn}`)
  }
}

function serializeIntent(intent: IntentData): Uint8Array {
  const w = new BcsWriter()
  writeVecU8(w, intent.module)
  writeVecU8(w, intent.function)
  writeU64(w, intent.nonce)
  w.writeULEB(intent.objects.length)
  for (const obj of intent.objects) {
    w.writeBytes(obj.id)
    w.write8(obj.mutable ? 1 : 0)
  }
  w.writeULEB(intent.requests.length)
  for (const req of intent.requests) {
    if (req.kind === 'uid') {
      w.writeULEB(0)
    } else if (req.kind === 'ip32') {
      w.writeULEB(1)
      w.writeBytes(req.domain!)
    } else if (req.kind === 'tripcode') {
      w.writeULEB(2)
    } else if (req.kind === 'geo') {
      w.writeULEB(3)
    } else if (req.kind === 'captcha') {
      w.writeULEB(4)
    }
  }
  writeVecU8(w, intent.payload)
  writeU256LE(w, intent.publicKey)
  writeU256LE(w, intent.tweak)

  return w.toBytes()
}

function writeU64(w: BcsWriter, v: number): void {
  const bytes = new Uint8Array(8)
  new DataView(bytes.buffer).setBigUint64(0, BigInt(v), true)
  w.writeBytes(bytes)
}

function writeU256LE(w: BcsWriter, bytes: Uint8Array): void {
  w.writeBytes(bytes)
}

function writeVecU8(w: BcsWriter, data: Uint8Array): void {
  w.writeULEB(data.length)
  w.writeBytes(data)
}

export class IntentBuilder {
  private _desc: EventDesc | null = null
  private _boardId?: Uint8Array
  private _threadId?: Uint8Array
  private _postId?: Uint8Array
  private _tweakArgs?: (Uint8Array | string)[]
  private _tweak?: Uint8Array
  private _requests: IntentRequest[] = [{ kind: 'uid' }]
  private _tripcode = false
  private _geo = false
  private _captcha = false

  constructor(
    private forumId: Uint8Array,
    private nonceShardsId: Uint8Array,
    private masterSecret: Uint8Array,
    private relayUrl: string,
  ) {}

  event(desc: EventDesc): this {
    this._desc = desc
    return this
  }

  board(id: Uint8Array): this {
    this._boardId = id
    return this
  }

  thread(id: Uint8Array): this {
    this._threadId = id
    return this
  }

  post(id: Uint8Array): this {
    this._postId = id
    return this
  }

  tweak(...args: (Uint8Array | string)[]): this {
    this._tweakArgs = args
    return this
  }

  rawTweak(bytes: Uint8Array): this {
    this._tweak = bytes
    return this
  }

  role(opt?: RoleOption | null): this {
    if (opt?.rawTweak) return this.rawTweak(opt.rawTweak)
    if (opt?.tweakArgs) return this.tweak(...opt.tweakArgs)
    return this
  }

  requests(...reqs: IntentRequest[]): this {
    this._requests = reqs
    return this
  }

  tripcode(): this {
    this._tripcode = true
    return this
  }

  geo(): this {
    this._geo = true
    return this
  }

  captcha(): this {
    this._captcha = true
    return this
  }

  deriveKeypair(): { secretKey: Uint8Array; publicKey: Uint8Array; tweak: Uint8Array } {
    const tweak = this._tweak
      ?? (this._tweakArgs
        ? computeTweak(...this._tweakArgs)
        : crypto.getRandomValues(new Uint8Array(32)))
    this._tweak = tweak
    const { secretKey, publicKey } = derivePostKeypair(this.masterSecret, this.forumId, tweak)
    return { secretKey, publicKey, tweak }
  }

  async build(nonceOverride?: number): Promise<{ intentBytes: Uint8Array; signature: Uint8Array }> {
    const desc = this._desc
    if (!desc) throw new Error('event not set')

    const fn = resolveFunction(desc, this._tripcode, this._geo, this._captcha, !!this._postId)
    const objects = resolveObjects(desc, this.forumId, this._boardId, this._threadId, this._postId, this._tripcode, this._geo, this._captcha)

    const { secretKey, publicKey, tweak } = this.deriveKeypair()

    const suiAddr = senderAddress(publicKey)
    const addrHash = blake2b(suiAddr, { dkLen: 32 })
    const sIdx = Number(new DataView(addrHash.buffer, addrHash.byteOffset + 24, 8).getBigUint64(0, false) % 512n)
    const shardId = fromHex(deriveShardId(toHex(this.nonceShardsId).replace(/^0x/, ''), sIdx))
    const shardIdx = objects.findIndex(o => o.id.length === 0)
    if (shardIdx < 0) throw new Error('missing nonce shard slot')
    objects[shardIdx].id = shardId

    const payload = new BcsWriter()
    const tagBytes = new TextEncoder().encode(desc.name)
    writeVecU8(payload, tagBytes)
    desc.writePayload(payload)

    const nonce = nonceOverride ?? await fetchNonce(this.relayUrl, toHex(publicKey).replace(/^0x/, ''))

    const requests: IntentRequest[] = [{ kind: 'uid' }]
    if (this._geo) requests.push({ kind: 'geo' })
    if (this._tripcode) requests.push({ kind: 'tripcode' })
    if (this._captcha) requests.push({ kind: 'captcha' })

    const intent: IntentData = {
      module: MODULE,
      function: new TextEncoder().encode(fn),
      nonce,
      objects,
      requests: this._geo || this._tripcode || this._captcha ? requests : this._requests,
      payload: payload.toBytes(),
      publicKey,
      tweak,
    }

    const intentBytes = serializeIntent(intent)
    const signature = await sign(secretKey, intentBytes)
    return { intentBytes, signature }
  }
}

export interface IntentBuilderCtx {
  forumId: string
  nonceShardsId: string
  relayUrl: string
  masterSecret?: Uint8Array
}

export function createIntentBuilder(ctx: IntentBuilderCtx): IntentBuilder {
  return new IntentBuilder(
    fromHex(ctx.forumId),
    fromHex(ctx.nonceShardsId),
    ctx.masterSecret ?? getSecretKeyBytes(),
    ctx.relayUrl,
  )
}
