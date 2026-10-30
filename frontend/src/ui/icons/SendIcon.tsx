import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function SendIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-send" glyph="->" />
}
