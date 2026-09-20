import { BcsWriter } from '@mysten/bcs'
import type { FieldKind } from './fields'

function writeString(w: BcsWriter, s: string): void {
  const b = new TextEncoder().encode(s)
  w.writeULEB(b.length)
  w.writeBytes(b)
}

function writeVecBytes32(w: BcsWriter, items: Uint8Array[]): void {
  w.writeULEB(items.length)
  for (const item of items) w.writeBytes(item)
}

function writeOptBytes32(w: BcsWriter, v: Uint8Array | null): void {
  if (v) { w.write8(1); w.writeBytes(v) } else { w.write8(0) }
}

function writeBanKey(w: BcsWriter, v: { level: Uint8Array; mask: number; ip_hash: Uint8Array }): void {
  w.writeBytes(v.level); w.write8(v.mask); w.writeBytes(v.ip_hash)
}

export const writer: Record<FieldKind, (w: BcsWriter, v: any) => void> = {
  u8: (w, v) => w.write8(v),
  bool: (w, v) => w.write8(v ? 1 : 0),
  u64: (w, v) => w.write64(v),
  string: (w, v) => writeString(w, v),
  u256: (w, v) => w.writeBytes(v),
  address: (w, v) => w.writeBytes(v),
  optU256: (w, v) => writeOptBytes32(w, v),
  optAddress: (w, v) => writeOptBytes32(w, v),
  vecU256: (w, v) => writeVecBytes32(w, v),
  vecAddress: (w, v) => writeVecBytes32(w, v),
  banKey: (w, v) => writeBanKey(w, v),
  optBanKey: (w, v) => { if (v) { w.write8(1); writeBanKey(w, v) } else { w.write8(0) } },
}
