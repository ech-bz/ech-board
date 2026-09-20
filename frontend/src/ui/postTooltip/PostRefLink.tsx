import { type JSX } from 'solid-js'
import { usePostTooltip, type PostTooltipTarget } from './usePostTooltip'

export function PostRefLink(props: {
  label: JSX.Element
  class?: string
  href?: string
  target: PostTooltipTarget
  onClick?: (e: MouseEvent) => void
}) {
  const { open, isOpen } = usePostTooltip()
  const coarse = window.matchMedia('(pointer: coarse)').matches
  return (
    <a
      class={props.class}
      href={props.href}
      onClick={(e) => {
        if (coarse) {
          if (!isOpen(e.currentTarget)) {
            e.preventDefault()
            open(e.currentTarget, props.target)
            return
          }
        }
        props.onClick?.(e)
      }}
      onMouseEnter={(e) => {
        if (!coarse) open(e.currentTarget, props.target)
      }}
    >
      {props.label}
    </a>
  )
}
