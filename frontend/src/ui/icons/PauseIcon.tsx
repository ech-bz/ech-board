import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function PauseIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-pause" glyph="||" filled />
}
