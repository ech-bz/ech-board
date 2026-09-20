import { BcsReader } from '@mysten/bcs'
import { toHex } from '../intent/crypto'
import type { FieldKind } from './fields'

export class TrackingReader extends BcsReader {
  pos = 0
  readonly len: number
  constructor(data: Uint8Array) {
    super(data)
    this.len = data.length
  }
  shift(bytes: number): this {
    this.pos += bytes
    return super.shift(bytes)
  }
  remaining(): Uint8Array {
    return this.readBytes(this.len - this.pos)
  }
}

export function readString(r: BcsReader): string {
  const len = r.readULEB()
  return new TextDecoder().decode(r.readBytes(len))
}

export function readBool(r: BcsReader): boolean {
  return r.read8() !== 0
}

export function readOption<T>(r: BcsReader, readVal: (r: BcsReader) => T): T | null {
  if (r.readULEB() === 0) return null
  return readVal(r)
}

export function readVec<T>(r: BcsReader, readVal: (r: BcsReader) => T): T[] {
  const len = r.readULEB()
  const result: T[] = []
  for (let i = 0; i < len; i++) result.push(readVal(r))
  return result
}

export function readSender(r: BcsReader): { pk: Uint8Array; tweak: Uint8Array } {
  return { pk: r.readBytes(32), tweak: r.readBytes(32) }
}

export function readResponses(r: BcsReader): { uid: Uint8Array | null; ip32: Uint8Array | null; tripcode: { secured: boolean; trip: string } | null; geo: number | null } {
  const uid = readOption(r, rr => { const len = rr.readULEB(); return rr.readBytes(len) })
  const ip32 = readOption(r, rr => rr.readBytes(32))
  const tripcode = readOption(r, rr => ({ secured: readBool(rr), trip: readString(rr) }))
  const geo = readOption(r, rr => rr.read32())
  return { uid, ip32, tripcode, geo }
}

function readBytes32Hex(r: BcsReader): string { return toHex(r.readBytes(32)) }
function readOptBytes32Hex(r: BcsReader): string | null { const v = readOption(r, rr => rr.readBytes(32)); return v ? toHex(v) : null }
function readVecBytes32Hex(r: BcsReader): string[] { return readVec(r, rr => toHex(rr.readBytes(32))) }
function readBanKey(r: BcsReader): { level: string; mask: number; ip_hash: string } {
  return { level: toHex(r.readBytes(32)), mask: r.read8(), ip_hash: toHex(r.readBytes(32)) }
}

export const reader: Record<FieldKind, (r: BcsReader) => any> = {
  u8: r => r.read8(),
  bool: readBool,
  u64: r => r.read64(),
  string: readString,
  u256: readBytes32Hex,
  address: readBytes32Hex,
  optU256: readOptBytes32Hex,
  optAddress: readOptBytes32Hex,
  vecU256: readVecBytes32Hex,
  vecAddress: readVecBytes32Hex,
  banKey: readBanKey,
  optBanKey: r => readOption(r, readBanKey),
}
