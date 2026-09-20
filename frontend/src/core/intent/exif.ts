import { transformExif } from '@uwx/exif-be-gone-web'

export function stripExif(bytes: Uint8Array): Promise<Uint8Array> {
  return transformExif(bytes)
}
