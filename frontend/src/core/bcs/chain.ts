import { bcs, type InferBcsType } from '@mysten/bcs'
import { Address, Sender, Table, Feed, Bans, BanKey, Tripcode, EntityRoot, versioned } from './common'

const BoardProjectionV1Schema = bcs.struct('BoardProjectionV1', {
  slug: bcs.string(),
  description_hash: bcs.option(Address),
  max_media: bcs.u64(),
  bump_limit: bcs.u64(),
  closed: bcs.bool(),
  deleted: bcs.bool(),
  ignore_forum_bans: bcs.bool(),
  mods: Table,
  bans: Bans,
  reactions: bcs.vector(Address),
  threads: Table,
  posts: Table,
  bumps: Feed,
})

const BoardProjectionV2Schema = bcs.struct('BoardProjectionV2', {
  slug: bcs.string(),
  description_hash: bcs.option(Address),
  max_media: bcs.u64(),
  bump_limit: bcs.u64(),
  closed: bcs.bool(),
  deleted: bcs.bool(),
  pinned: bcs.vector(Address),
  ignore_forum_bans: bcs.bool(),
  mods: Table,
  bans: Bans,
  reactions: bcs.vector(Address),
  threads: Table,
  posts: Table,
  bumps: Feed,
})

const BoardProjectionSchema = bcs.enum('BoardProjection', {
  V1: BoardProjectionV1Schema,
  V2: BoardProjectionV2Schema,
})

type BoardProjectionV1Data = InferBcsType<typeof BoardProjectionV1Schema>
type BoardProjectionV2Data = InferBcsType<typeof BoardProjectionV2Schema>
export type BoardProjectionData = InferBcsType<typeof BoardProjectionSchema>

export class BoardProjection {
  private version: number
  private data: BoardProjectionV1Data | BoardProjectionV2Data
  constructor(raw: BoardProjectionData) {
    switch (Object.keys(raw)[0]) {
      case 'V1': this.version = 1; this.data = raw.V1!; break
      case 'V2': this.version = 2; this.data = raw.V2!; break
      default: throw new Error('unsupported board projection version')
    }
  }
  slug() { return this.data.slug }
  description_hash() { return this.data.description_hash }
  max_media() { return this.data.max_media }
  bump_limit() { return this.data.bump_limit }
  closed() { return this.data.closed }
  deleted() { return this.data.deleted }
  ignore_forum_bans() { return this.data.ignore_forum_bans }
  mods() { return this.data.mods }
  bans() { return this.data.bans }
  reactions() { return this.data.reactions }
  threads() { return this.data.threads }
  posts() { return this.data.posts }
  bumps() { return this.data.bumps }
  pinned() { return this.version === 2 ? (this.data as BoardProjectionV2Data).pinned : [] }
}

const BoardObjectSchema = bcs.struct('BoardObject', {
  root: EntityRoot,
  projection: BoardProjectionSchema,
})

export type BoardObject = InferBcsType<typeof BoardObjectSchema>

export const BoardObject = versioned<BoardObject>(BoardObjectSchema, [1, 2])

const ThreadProjectionV1Schema = bcs.struct('ThreadProjectionV1', {
  board: Address,
  number: bcs.u64(),
  topic_hash: bcs.option(Address),
  op: Address,
  closed: bcs.bool(),
  deleted: bcs.bool(),
  pinned: bcs.bool(),
  admin: bcs.option(Address),
  mods: Table,
  bans: Bans,
  posts: Feed,
  last_3: bcs.vector(Address),
})

const ThreadProjectionV2Schema = bcs.struct('ThreadProjectionV2', {
  board: Address,
  number: bcs.u64(),
  topic_hash: bcs.option(Address),
  op: Address,
  closed: bcs.bool(),
  deleted: bcs.bool(),
  admin: bcs.option(Address),
  mods: Table,
  bans: Bans,
  posts: Feed,
  last_3: bcs.vector(Address),
})

const ThreadProjectionV3Schema = bcs.struct('ThreadProjectionV3', {
  board: Address,
  number: bcs.u64(),
  topic_hash: bcs.option(Address),
  op: Address,
  closed: bcs.bool(),
  deleted: bcs.bool(),
  admin: bcs.option(Address),
  mods: Table,
  bans: Bans,
  posts: Feed,
  posts_deleted: bcs.u64(),
  last_3: bcs.vector(Address),
})

const ThreadProjectionSchema = bcs.enum('ThreadProjection', {
  V1: ThreadProjectionV1Schema,
  V2: ThreadProjectionV2Schema,
  V3: ThreadProjectionV3Schema,
})

type ThreadProjectionV1Data = InferBcsType<typeof ThreadProjectionV1Schema>
type ThreadProjectionV2Data = InferBcsType<typeof ThreadProjectionV2Schema>
type ThreadProjectionV3Data = InferBcsType<typeof ThreadProjectionV3Schema>
export type ThreadProjectionData = InferBcsType<typeof ThreadProjectionSchema>

export class ThreadProjection {
  private version: number
  private data: ThreadProjectionV1Data | ThreadProjectionV2Data | ThreadProjectionV3Data
  constructor(raw: ThreadProjectionData) {
    switch (Object.keys(raw)[0]) {
      case 'V1': this.version = 1; this.data = raw.V1!; break
      case 'V2': this.version = 2; this.data = raw.V2!; break
      case 'V3': this.version = 3; this.data = raw.V3!; break
      default: throw new Error('unsupported thread projection version')
    }
  }
  board() { return this.data.board }
  number() { return this.data.number }
  topic_hash() { return this.data.topic_hash }
  op() { return this.data.op }
  closed() { return this.data.closed }
  deleted() { return this.data.deleted }
  admin() { return this.data.admin }
  mods() { return this.data.mods }
  bans() { return this.data.bans }
  posts() { return this.data.posts }
  last_3() { return this.data.last_3 }
  pinned() { return this.version === 1 ? (this.data as ThreadProjectionV1Data).pinned : false }
  posts_deleted() { return this.version === 3 ? (this.data as ThreadProjectionV3Data).posts_deleted : null }
}

const ThreadObjectSchema = bcs.struct('ThreadObject', {
  root: EntityRoot,
  projection: ThreadProjectionSchema,
})

export type ThreadObject = InferBcsType<typeof ThreadObjectSchema>

export const ThreadObject = versioned<ThreadObject>(ThreadObjectSchema, [1, 2, 3])

const PostProjectionV1Schema = bcs.struct('PostProjectionV1', {
  sender: Sender,
  thread: Address,
  number: bcs.u64(),
  uid: bcs.byteVector(),
  timestamp_ms: bcs.u64(),
  deleted: bcs.bool(),
  banned: bcs.option(BanKey),
  text_hash: bcs.option(Address),
  media_hashes: bcs.vector(Address),
  reactions: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  votes: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  name_hash: bcs.option(Address),
  trip: bcs.option(Tripcode),
  geo: bcs.option(bcs.u32()),
  mod_note: bcs.option(Address),
})

const PostProjectionV2Schema = bcs.struct('PostProjectionV2', {
  sender: Sender,
  thread: Address,
  number: bcs.u64(),
  uid: bcs.byteVector(),
  timestamp_ms: bcs.u64(),
  deleted: bcs.bool(),
  banned: bcs.option(BanKey),
  text_hash: bcs.option(Address),
  media_hashes: bcs.vector(Address),
  reactions: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  votes: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  multi_vote: bcs.bool(),
  name_hash: bcs.option(Address),
  trip: bcs.option(Tripcode),
  geo: bcs.option(bcs.u32()),
  mod_note: bcs.option(Address),
})

const PostProjectionV3Schema = bcs.struct('PostProjectionV3', {
  sender: Sender,
  thread: Address,
  number: bcs.u64(),
  uid: bcs.byteVector(),
  timestamp_ms: bcs.u64(),
  deleted: bcs.bool(),
  banned: bcs.option(BanKey),
  text_hash: bcs.option(Address),
  media_hashes: bcs.vector(Address),
  banned_media: bcs.vector(Address),
  reactions: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  votes: bcs.vector(bcs.tuple([Address, bcs.u64()])),
  multi_vote: bcs.bool(),
  name_hash: bcs.option(Address),
  trip: bcs.option(Tripcode),
  geo: bcs.option(bcs.u32()),
  mod_note: bcs.option(Address),
})

const PostProjectionSchema = bcs.enum('PostProjection', {
  V1: PostProjectionV1Schema,
  V2: PostProjectionV2Schema,
  V3: PostProjectionV3Schema,
})

type PostProjectionV1Data = InferBcsType<typeof PostProjectionV1Schema>
type PostProjectionV2Data = InferBcsType<typeof PostProjectionV2Schema>
type PostProjectionV3Data = InferBcsType<typeof PostProjectionV3Schema>
export type PostProjectionData = InferBcsType<typeof PostProjectionSchema>

export class PostProjection {
  private version: number
  private data: PostProjectionV1Data | PostProjectionV2Data | PostProjectionV3Data
  constructor(raw: PostProjectionData) {
    switch (Object.keys(raw)[0]) {
      case 'V1': this.version = 1; this.data = raw.V1!; break
      case 'V2': this.version = 2; this.data = raw.V2!; break
      case 'V3': this.version = 3; this.data = raw.V3!; break
      default: throw new Error('unsupported post projection version')
    }
  }
  sender() { return this.data.sender }
  thread() { return this.data.thread }
  number() { return this.data.number }
  uid() { return this.data.uid }
  timestamp_ms() { return this.data.timestamp_ms }
  deleted() { return this.data.deleted }
  banned() { return this.data.banned }
  text_hash() { return this.data.text_hash }
  media_hashes() { return this.data.media_hashes }
  reactions() { return this.data.reactions }
  votes() { return this.data.votes }
  multi_vote() { return this.version >= 2 ? (this.data as PostProjectionV2Data | PostProjectionV3Data).multi_vote : false }
  name_hash() { return this.data.name_hash }
  trip() { return this.data.trip }
  geo() { return this.data.geo }
  mod_note() { return this.data.mod_note }
  banned_media() { return this.version === 3 ? (this.data as PostProjectionV3Data).banned_media : [] }
}

const PostObjectSchema = bcs.struct('PostObject', {
  root: EntityRoot,
  projection: PostProjectionSchema,
})

export type PostObject = InferBcsType<typeof PostObjectSchema>

export const PostObject = versioned<PostObject>(PostObjectSchema, [1, 2, 3])

const ForumProjectionV1Schema = bcs.struct('ForumProjectionV1', {
  nonce_shards: Address,
  admin: Address,
  mods: Table,
  bans: Bans,
  boards: Table,
  timestamp_precision_ms: bcs.u64(),
})

const ForumProjectionSchema = bcs.enum('ForumProjection', {
  V1: ForumProjectionV1Schema,
})

type ForumProjectionV1Data = InferBcsType<typeof ForumProjectionV1Schema>
export type ForumProjectionData = InferBcsType<typeof ForumProjectionSchema>

export class ForumProjection {
  private data: ForumProjectionV1Data
  constructor(raw: ForumProjectionData) {
    switch (Object.keys(raw)[0]) {
      case 'V1': this.data = raw.V1!; break
      default: throw new Error('unsupported forum projection version')
    }
  }
  nonce_shards() { return this.data.nonce_shards }
  admin() { return this.data.admin }
  mods() { return this.data.mods }
  bans() { return this.data.bans }
  boards() { return this.data.boards }
  timestamp_precision_ms() { return this.data.timestamp_precision_ms }
}

const ForumObjectSchema = bcs.struct('ForumObject', {
  root: EntityRoot,
  projection: ForumProjectionSchema,
})

export type ForumObject = InferBcsType<typeof ForumObjectSchema>

export const ForumObject = versioned<ForumObject>(ForumObjectSchema, [1])
