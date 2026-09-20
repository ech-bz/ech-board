import { blake2b } from '@noble/hashes/blake2'
import { sha512 } from '@noble/hashes/sha512'
import * as ed25519 from '@noble/ed25519'

ed25519.etc.sha512Sync = sha512

export function generateKeyPair(): { publicKey: Uint8Array; secretKey: Uint8Array } {
  const secretKey = ed25519.utils.randomPrivateKey()
  const publicKey = ed25519.getPublicKey(secretKey)
  return { publicKey, secretKey }
}

export function derivePostKeypair(masterSecret: Uint8Array, forumId: Uint8Array, tweak: Uint8Array): { publicKey: Uint8Array; secretKey: Uint8Array } {
  const seed = new Uint8Array(masterSecret.length + forumId.length + tweak.length)
  seed.set(masterSecret)
  seed.set(forumId, masterSecret.length)
  seed.set(tweak, masterSecret.length + forumId.length)
  const derived = blake2b(seed, { dkLen: 32 })
  return { secretKey: derived, publicKey: ed25519.getPublicKey(derived) }
}

export function computeTweak(...args: (Uint8Array | string)[]): Uint8Array {
  const encoder = new TextEncoder()
  const parts = args.map(a =>
    typeof a === 'string' ? blake2b(encoder.encode(a), { dkLen: 32 }) : blake2b(a, { dkLen: 32 }),
  )
  const concat = new Uint8Array(parts.length * 32)
  parts.forEach((p, i) => concat.set(p, i * 32))
  return blake2b(concat, { dkLen: 32 })
}

export async function sign(secretKey: Uint8Array, message: Uint8Array): Promise<Uint8Array> {
  const hash = blake2b(message, { dkLen: 32 })
  return ed25519.signAsync(hash, secretKey)
}

export function publicKeyFromSecret(secretKey: Uint8Array): Uint8Array {
  return ed25519.getPublicKey(secretKey)
}

export function senderAddress(publicKey: Uint8Array): Uint8Array {
  const data = new Uint8Array(33)
  data[0] = 0x00
  data.set(publicKey, 1)
  return blake2b(data, { dkLen: 32 })
}

export function shardIndex(address: Uint8Array, shardCount: number): number {
  const hash = blake2b(address, { dkLen: 32 })
  const view = new DataView(hash.buffer, hash.byteOffset + 24, 8)
  const val = view.getBigUint64(0, false)
  return Number(val % BigInt(shardCount))
}

export function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((n, a) => n + a.length, 0)
  const out = new Uint8Array(total)
  let off = 0
  for (const a of arrays) {
    out.set(a, off)
    off += a.length
  }
  return out
}

const HEX_DIGITS = '0123456789abcdef'

export function toHex(bytes: Uint8Array): string {
  let out = '0x'
  for (let i = 0; i < bytes.length; i++) out += HEX_DIGITS[bytes[i] >> 4] + HEX_DIGITS[bytes[i] & 15]
  return out
}

export function fromHex(hex: string): Uint8Array {
  const stripped = hex.replace(/^0x/, '')
  const bytes = new Uint8Array(stripped.length / 2)
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(stripped.substring(i * 2, i * 2 + 2), 16)
  }
  return bytes
}

