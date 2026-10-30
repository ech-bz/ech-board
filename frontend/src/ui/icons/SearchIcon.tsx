import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function SearchIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-search" glyph="?" />
}
