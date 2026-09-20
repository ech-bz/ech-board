import { bcs } from '@mysten/bcs'
import { deriveObjectID } from '@mysten/sui/utils'

export function deriveShardId(nonceShardsId: string, shardIndex: number): string {
  return deriveObjectID(nonceShardsId, 'u64', bcs.u64().serialize(shardIndex).toBytes())
}

