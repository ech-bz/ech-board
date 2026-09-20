import { bcs, type InferBcsType } from '@mysten/bcs'
import { Address } from './common'
import { BoardObject, ThreadObject, PostObject, ForumObject } from './chain'

const AddressVecPostObject = bcs.vector(bcs.tuple([Address, bcs.vector(PostObject)]))

const AddressPostObjectMap = bcs.vector(bcs.tuple([Address, PostObject]))

const AddressBytesMap = bcs.vector(bcs.tuple([Address, bcs.vector(bcs.u8())]))

export const MediaMeta = bcs.struct('MediaMeta', {
  mime: bcs.string(),
  width: bcs.u32(),
  height: bcs.u32(),
  duration_ms: bcs.option(bcs.u64()),
  size: bcs.u64(),
})

export type MediaMetaData = InferBcsType<typeof MediaMeta>

const AddressMediaMetaMap = bcs.vector(bcs.tuple([Address, MediaMeta]))

export const Moderators = bcs.struct('Moderators', {
  forum_admin: bcs.option(Address),
  forum_mods: bcs.vector(Address),
  board_mods: bcs.vector(Address),
  thread_mods: bcs.vector(Address),
  thread_admin: bcs.option(Address),
})

export type ModeratorsData = InferBcsType<typeof Moderators>

export const BoardView = bcs.struct('BoardView', {
  board: BoardObject,
  threads: bcs.vector(ThreadObject),
  op_posts: AddressPostObjectMap,
  last_3: AddressVecPostObject,
  text: AddressBytesMap,
  plain_text: AddressBytesMap,
  media_meta: AddressMediaMetaMap,
  next_cursor: bcs.option(bcs.u64()),
  moderators: Moderators,
})

export const ThreadView = bcs.struct('ThreadView', {
  thread: ThreadObject,
  posts: bcs.vector(PostObject),
  text: AddressBytesMap,
  plain_text: AddressBytesMap,
  media_meta: AddressMediaMetaMap,
  moderators: Moderators,
})

export const PostReactions = bcs.struct('PostReactions', {
  reaction: bcs.option(Address),
})

export const ReactionsBatch = bcs.vector(bcs.tuple([Address, Address]))

export const PostView = bcs.struct('PostView', {
  post: PostObject,
  thread: ThreadObject,
  board: BoardObject,
  text: AddressBytesMap,
  plain_text: AddressBytesMap,
  media_meta: AddressMediaMetaMap,
  moderators: Moderators,
})

export const ForumView = bcs.struct('ForumView', {
  forum: ForumObject,
  boards: bcs.vector(BoardObject),
  plain_text: AddressBytesMap,
  moderators: Moderators,
})

export const NonceInfo = bcs.struct('NonceInfo', {
  nonce: bcs.u64(),
})

const RelayEvent = bcs.struct('RelayEvent', {
  package_id: bcs.string(),
  module: bcs.string(),
  sender: bcs.string(),
  event_type: bcs.string(),
  contents: bcs.vector(bcs.u8()),
})

export const SendResponse = bcs.struct('SendResponse', {
  accepted_by: bcs.vector(bcs.string()),
  digest: bcs.string(),
  events: bcs.vector(RelayEvent),
  created: bcs.vector(Address),
})

export const FeedView = bcs.struct('FeedView', {
  items: bcs.vector(bcs.vector(bcs.u8())),
  next_cursor: bcs.option(bcs.u64()),
})

const BanEntry = bcs.struct('BanEntry', {
  mask: bcs.u8(),
  ip_hash: Address,
  reason_hash: Address,
  reason: bcs.option(bcs.string()),
  expires: bcs.u64(),
})

export const BansView = bcs.struct('BansView', {
  level: Address,
  bans: bcs.vector(BanEntry),
  next_cursor: bcs.option(bcs.u64()),
})
