import { Show } from 'solid-js'
import { useIsPotato } from '../i18n'
import type { IconProps } from './IconProps'

interface Props extends IconProps {
  href: string
  glyph: string
  filled?: boolean
}

export function AppIcon(props: Props) {
  const potato = useIsPotato()
  const iconWidth = () => Number.parseFloat(String(props.width ?? 24)) || 24
  const iconHeight = () => Number.parseFloat(String(props.height ?? 24)) || 24
  const hasSizeClass = () => /(^|\s)(size|w|h)-/.test(props.class ?? '')
  return (
    <Show
      when={!potato()}
      fallback={
        <span
          class={`potato-ascii-icon ${props.class ?? ''}`}
          data-icon-size={!hasSizeClass() ? `${iconWidth()}x${iconHeight()}` : undefined}
          aria-hidden="true"
        >
          {props.glyph}
        </span>
      }
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={props.width ?? 24}
        height={props.height ?? 24}
        viewBox="0 0 24 24"
        fill={props.filled ? 'currentColor' : 'none'}
        stroke={props.filled ? undefined : 'currentColor'}
        stroke-width={props.filled ? undefined : 2}
        stroke-linecap={props.filled ? undefined : 'round'}
        stroke-linejoin={props.filled ? undefined : 'round'}
        class={props.class}
      >
        <use href={props.href} />
      </svg>
    </Show>
  )
}
