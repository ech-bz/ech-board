import type { PostPartInput } from '../bcs/types'
import { ed25519, x25519 } from '@noble/curves/ed25519.js'

async function generateMasterKey(): Promise<CryptoKey> {
  return crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt'],
  )
}

async function encryptContent(key: CryptoKey, data: Uint8Array): Promise<{ nonce: Uint8Array; ct: Uint8Array }> {
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const ct = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: nonce },
    key,
    data,
  )
  return { nonce, ct: new Uint8Array(ct) }
}

async function wrapMasterKey(
  masterKey: CryptoKey,
  senderPriv: Uint8Array,
  recipientPub: Uint8Array,
): Promise<{ nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array }> {
  const senderMont = ed25519.utils.toMontgomerySecret(senderPriv)
  const recipMont = ed25519.utils.toMontgomery(recipientPub)
  const shared = x25519.getSharedSecret(senderMont, recipMont)

  const rawKey = await crypto.subtle.exportKey('raw', masterKey)
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const wrapKey = await crypto.subtle.importKey(
    'raw',
    shared.slice(0, 32),
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt'],
  )
  const wrapped = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: nonce },
    wrapKey,
    rawKey,
  )
  const buf = new Uint8Array(wrapped)
  return { nonce, ct: buf.slice(0, 32), tag: buf.slice(32) }
}

export async function encryptSecret(
  data: Uint8Array,
  senderPriv: Uint8Array,
  senderPub: Uint8Array,
  recipientPubs: Uint8Array[],
): Promise<PostPartInput> {
  const masterKey = await generateMasterKey()
  const { nonce: dataNonce, ct: dataCt } = await encryptContent(masterKey, data)

  const allPubs = [senderPub]
  for (const pk of recipientPubs) {
    if (!allPubs.some(p => p.length === pk.length && p.every((b, i) => b === pk[i]))) {
      allPubs.push(pk)
    }
  }

  const encryptedKeys: { nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array }[] = []
  for (const pub of allPubs) {
    encryptedKeys.push(await wrapMasterKey(masterKey, senderPriv, pub))
  }

  return { Secret: { data_nonce: dataNonce, data_ct: dataCt, encrypted_keys: encryptedKeys } }
}

async function unwrapMasterKey(
  ek: { nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array },
  myPriv: Uint8Array,
  senderPub: Uint8Array,
): Promise<CryptoKey> {
  const myMont = ed25519.utils.toMontgomerySecret(myPriv)
  const senderMont = ed25519.utils.toMontgomery(senderPub)
  const shared = x25519.getSharedSecret(myMont, senderMont)

  const wrapKey = await crypto.subtle.importKey(
    'raw',
    shared.slice(0, 32),
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt'],
  )

  const combined = new Uint8Array(ek.ct.length + ek.tag.length)
  combined.set(ek.ct)
  combined.set(ek.tag, ek.ct.length)

  const rawKey = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: ek.nonce },
    wrapKey,
    combined,
  )

  return crypto.subtle.importKey('raw', rawKey, { name: 'AES-GCM', length: 256 }, true, ['decrypt'])
}

async function decryptContent(
  key: CryptoKey,
  nonce: Uint8Array,
  ct: Uint8Array,
): Promise<Uint8Array> {
  const pt = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: nonce },
    key,
    ct,
  )
  return new Uint8Array(pt)
}

export async function tryDecryptSecret(
  dataNonce: Uint8Array,
  dataCt: Uint8Array,
  encryptedKeys: { nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array }[],
  myPriv: Uint8Array,
  senderPub: Uint8Array,
): Promise<Uint8Array | null> {
  for (let i = 0; i < encryptedKeys.length; i++) {
    const ek = encryptedKeys[i]
    try {
      const masterKey = await unwrapMasterKey(ek, myPriv, senderPub)
      return await decryptContent(masterKey, dataNonce, dataCt)
    } catch (e) {
      continue
    }
  }
  return null
}
