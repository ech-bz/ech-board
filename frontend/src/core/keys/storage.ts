import { generateKeyPair, toHex, fromHex, publicKeyFromSecret } from '../intent/crypto'

const KEYS_KEY = 'ech-board-keypair'

interface KeyPair {
  publicKey: string
  secretKey: string
}

export function getOrCreateKeyPair(): KeyPair {
  const stored = localStorage.getItem(KEYS_KEY)
  if (stored) {
    return JSON.parse(stored)
  }
  const { publicKey, secretKey } = generateKeyPair()
  const kp: KeyPair = {
    publicKey: toHex(publicKey),
    secretKey: toHex(secretKey),
  }
  localStorage.setItem(KEYS_KEY, JSON.stringify(kp))
  return kp
}

export function getPublicKeyBytes(): Uint8Array {
  return fromHex(getOrCreateKeyPair().publicKey)
}

export function getSecretKeyBytes(): Uint8Array {
  return fromHex(getOrCreateKeyPair().secretKey)
}

export function importSecretKey(secretKeyHex: string): void {
  const secretKeyBytes = fromHex(secretKeyHex)
  if (secretKeyBytes.length !== 32) throw new Error('invalid secret key length')
  const publicKey = toHex(publicKeyFromSecret(secretKeyBytes))
  localStorage.setItem(KEYS_KEY, JSON.stringify({ publicKey, secretKey: secretKeyHex }))
}
