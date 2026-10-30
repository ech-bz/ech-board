import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function PinIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-pin" glyph="!" />
}
