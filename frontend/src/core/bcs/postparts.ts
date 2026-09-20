import { bcs, BcsType } from '@mysten/bcs'
import { lazy, Address } from './common'

export const PostPart: BcsType<PostPartInput> = bcs.enum('PostPart', {
  Plain: bcs.string(),
  Bold: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Italic: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Code: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  ReplyTo: bcs.struct('ReplyTo', {
    forum: Address,
    board: Address,
    post: Address,
    hint: bcs.string(),
  }),
  Secret: bcs.struct('Secret', {
    data_nonce: bcs.bytes(12),
    data_ct: bcs.vector(bcs.u8()),
    encrypted_keys: bcs.vector(bcs.struct('EncryptedKey', {
      nonce: bcs.bytes(12),
      ct: bcs.bytes(32),
      tag: bcs.bytes(16),
    })),
  }),
  Underline: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Overline: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Spoiler: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Strike: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Sup: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Sub: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  Link: bcs.struct('Link', {
    url: bcs.string(),
    children: bcs.vector(lazy(() => PostPart as unknown as BcsType<PostPartInput>)),
  }),
}) as unknown as BcsType<PostPartInput>

export type PostPartInput =
  | { Plain: string }
  | { Bold: PostPartInput[] }
  | { Italic: PostPartInput[] }
  | { Code: PostPartInput[] }
  | { ReplyTo: { forum: Uint8Array; board: Uint8Array; post: Uint8Array; hint: string } }
  | { Secret: { data_nonce: Uint8Array; data_ct: Uint8Array; encrypted_keys: { nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array }[] } }
  | { Underline: PostPartInput[] }
  | { Overline: PostPartInput[] }
  | { Spoiler: PostPartInput[] }
  | { Strike: PostPartInput[] }
  | { Sup: PostPartInput[] }
  | { Sub: PostPartInput[] }
  | { Link: { url: string; children: PostPartInput[] } }

export const PostPartVec = bcs.vector(PostPart)

export function serializePostParts(parts: PostPartInput[]): Uint8Array {
  return PostPartVec.serialize(parts).toBytes()
}
