import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function LockIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-lock" glyph="[x]" />
}
