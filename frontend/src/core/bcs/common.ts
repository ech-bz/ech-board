import { bcs, BcsType } from '@mysten/bcs'

export function lazy<T>(cb: () => BcsType<T>): BcsType<T> {
  let cached: BcsType<T> | null = null
  const get = (): BcsType<T> => {
    if (!cached) cached = cb()
    return cached
  }
  return new BcsType<T>({
    name: 'lazy' as never,
    read: (r) => get().read(r),
    write: (v, w) => get().write(v, w),
    serialize: (v, opts) => get().serialize(v, opts).toBytes(),
    serializedSize: (v) => get().serializedSize(v),
  })
}

export function versioned<U extends { root: { entity: { version: number } } }>(
  schema: BcsType<U, any>,
  supported: readonly U['root']['entity']['version'][],
): BcsType<U> {
  return new BcsType<U>({
    name: schema.name as never,
    read: (r) => {
      const v = schema.read(r) as U
      if (!supported.includes(v.root.entity.version)) {
        throw new Error(`unsupported ${schema.name} version ${v.root.entity.version}`)
      }
      return v
    },
    write: (v, w) => schema.write(v, w),
    serialize: (v, opts) => schema.serialize(v, opts).toBytes(),
    serializedSize: (v) => schema.serializedSize(v),
  })
}

export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
}

export const Address = bcs.bytes(32)
export const U256 = bcs.bytes(32)

export const Sender = bcs.struct('Sender', {
  pk: U256,
  tweak: U256,
})

export const Table = bcs.struct('Table', {
  id: Address,
  size: bcs.u64(),
})

export const Feed = bcs.struct('Feed', {
  id: Address,
  counter: bcs.u64(),
})

export const Entity = bcs.struct('Entity', {
  feed: Feed,
  version: bcs.u16(),
})

export const EntityRoot = bcs.struct('EntityRoot', {
  id: Address,
  entity: Entity,
  genesis: bcs.bool(),
})

export const Registry = bcs.struct('Registry', {
  counter: bcs.u64(),
  entries: Table,
  identities: Table,
  index: Table,
})

export const Bans = bcs.struct('Bans', {
  level: Address,
  ip32: Registry,
  ip24: Registry,
  ip20: Registry,
  ip16: Registry,
})

export const BanKey = bcs.struct('BanKey', {
  level: Address,
  mask: bcs.u8(),
  ip_hash: Address,
})

export const Tripcode = bcs.struct('Tripcode', {
  secured: bcs.bool(),
  trip: bcs.string(),
})
