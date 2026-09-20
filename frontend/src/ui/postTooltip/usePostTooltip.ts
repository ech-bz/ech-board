import { useContext } from 'solid-js'
import { createContext } from 'solid-js'
import type { TooltipPost } from '../../core/load'

export interface PostTooltipTarget {
  postUid: string
  number?: number
}

export type ResolvePost = (postUid: string) => Promise<TooltipPost | null>

export interface Api {
  open: (anchor: HTMLElement, target: PostTooltipTarget) => void
  isOpen: (anchor: HTMLElement) => boolean
}

export const Ctx = createContext<Api>({ open: () => {}, isOpen: () => false })

export function usePostTooltip(): Api {
  return useContext(Ctx)
}
