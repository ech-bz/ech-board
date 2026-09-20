import { BcsWriter, BcsReader } from '@mysten/bcs'
import { TrackingReader, reader, readString, readSender, readResponses } from '../bcs/reader'
import { writer } from '../bcs/writer'
import type { Field } from '../bcs/fields'
import { toHex, fromHex, senderAddress } from '../intent/crypto'

export type Ns = 'forum' | 'board' | 'thread' | 'post'

export interface EventDesc {
  name: string
  _ns: Ns
  writePayload(w: BcsWriter): void
}

export interface DecodedEvent {
  $kind: string
  version: number
  responses: {
    uid: string | null
    ip32: string | null
    tripcode: { secured: boolean; trip: string } | null
    geo: number | null
  }
  sender: { pk: string; tweak: string; address: string }
  payload: Record<string, any>
}

export interface SseEntity {
  kind: Ns
  id: string
  feed: string
  version: number
}

export interface EventDef {
  ns: Ns
  name: string
  fields: Field[]
  encode(...args: any[]): EventDesc
  decode(r: BcsReader): Record<string, any>
}

export function defineEvent(ns: Ns, name: string, fields: Field[]): EventDef {
  return {
    ns,
    name,
    fields,
    encode(...args: any[]): EventDesc {
      return {
        name,
        _ns: ns,
        writePayload(w: BcsWriter) {
          for (let i = 0; i < fields.length; i++) writer[fields[i].type](w, args[i])
        },
      }
    },
    decode(r: BcsReader): Record<string, any> {
      const out: Record<string, any> = {}
      for (const field of fields) out[field.key] = reader[field.type](r)
      return out
    },
  }
}

export const forumEvents = new Map<string, EventDef>()
export const boardEvents = new Map<string, EventDef>()
export const threadEvents = new Map<string, EventDef>()
export const postEvents = new Map<string, EventDef>()

const registries: Record<Ns, Map<string, EventDef>> = {
  forum: forumEvents,
  board: boardEvents,
  thread: threadEvents,
  post: postEvents,
}

export function parseEvent(bytes: Uint8Array, ns: Ns): DecodedEvent {
  const r = new TrackingReader(bytes)
  const version = r.read16()
  const responses = readResponses(r)
  const sender = readSender(r)
  const name = readString(r)
  const def = registries[ns].get(name)
  const payload = def ? def.decode(r) : { rest: toHex(r.remaining()) }
  return {
    $kind: name,
    version,
    responses: {
      uid: responses.uid ? toHex(responses.uid) : null,
      ip32: responses.ip32 ? toHex(responses.ip32) : null,
      tripcode: responses.tripcode,
      geo: responses.geo ?? null,
    },
    sender: {
      pk: toHex(sender.pk),
      tweak: toHex(sender.tweak),
      address: toHex(senderAddress(sender.pk)),
    },
    payload,
  }
}

export function decodeRealtimeEvent(kind: Ns, bytesHex: string): DecodedEvent {
  return parseEvent(fromHex(bytesHex), kind)
}

export function registerEvents(ns: Ns, defs: EventDef[]): void {
  const registry = registries[ns]
  for (const def of defs) registry.set(def.name, def)
}
