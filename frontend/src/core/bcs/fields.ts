export type FieldKind =
  | 'u8' | 'bool' | 'u64' | 'string'
  | 'u256' | 'address' | 'optU256' | 'optAddress'
  | 'vecU256' | 'vecAddress' | 'banKey' | 'optBanKey'

export interface Field {
  key: string
  type: FieldKind
}

export const f = {
  u8: (key: string): Field => ({ key, type: 'u8' }),
  bool: (key: string): Field => ({ key, type: 'bool' }),
  u64: (key: string): Field => ({ key, type: 'u64' }),
  string: (key: string): Field => ({ key, type: 'string' }),
  u256: (key: string): Field => ({ key, type: 'u256' }),
  address: (key: string): Field => ({ key, type: 'address' }),
  optU256: (key: string): Field => ({ key, type: 'optU256' }),
  optAddress: (key: string): Field => ({ key, type: 'optAddress' }),
  vecU256: (key: string): Field => ({ key, type: 'vecU256' }),
  vecAddress: (key: string): Field => ({ key, type: 'vecAddress' }),
  banKey: (key: string): Field => ({ key, type: 'banKey' }),
  optBanKey: (key: string): Field => ({ key, type: 'optBanKey' }),
}
