import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function ChevronLeftIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-chevron-left" glyph="<" />
}
