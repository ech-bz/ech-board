import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function EllipsisIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-ellipsis" glyph="..." />
}
