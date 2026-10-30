import type { IconProps } from './IconProps'
import { AppIcon } from './AppIcon'

export function PlayIcon(props: IconProps) {
  return <AppIcon {...props} href="#icon-play" glyph=">" filled />
}
