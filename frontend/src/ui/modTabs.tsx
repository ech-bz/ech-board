import { createSignal, type JSX } from 'solid-js'
import { ScanEyeIcon } from './icons/ScanEyeIcon'
import { FlagOffIcon } from './icons/FlagOffIcon'
import { GavelIcon } from './icons/GavelIcon'
import { ImageIcon } from './icons/ImageIcon'
import type { IconProps } from './icons/IconProps'
import type { MessageKey } from './i18n/ru'
import type { ModTab } from './route'

export const MOD_TABS: { tab: ModTab; label: MessageKey; Icon: (props: IconProps) => JSX.Element }[] = [
  { tab: 'overboard', label: 'modUi.overboard', Icon: ScanEyeIcon },
  { tab: 'reports', label: 'modUi.reports', Icon: FlagOffIcon },
  { tab: 'actions', label: 'modUi.actions', Icon: GavelIcon },
  { tab: 'gallery', label: 'modUi.gallery', Icon: ImageIcon },
]

const [open, setOpen] = createSignal(false)

export const modTabsOpen = open
export const openModTabs = () => setOpen(true)
export const closeModTabs = () => setOpen(false)
