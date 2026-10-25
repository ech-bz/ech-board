import { createContext } from 'solid-js'
import type { MediaInfo } from '../../../core/media'

export interface GalleryInfo {
  posterUid: string
  postNum: number
  board: string
  ts: number
  onOpenPost?: () => void
}

export type GalleryItem = MediaInfo & { info?: GalleryInfo }

export interface SourceRect {
  left: number
  top: number
  width: number
  height: number
}

export interface GalleryApi {
  open: (items: GalleryItem[], index: number, sourceRect?: SourceRect) => void
  close: () => void
}

export const ApiCtx = createContext<GalleryApi>({ open: () => {}, close: () => {} })
