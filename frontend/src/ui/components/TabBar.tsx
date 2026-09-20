import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { BoardsIcon } from '../icons/BoardsIcon'
import { SettingsIcon } from '../icons/SettingsIcon'
import { IconButton } from './IconButton'

const tabsVariants = cva('glass-fade glass-fade--up fixed inset-x-0 bottom-0 z-tabbar flex items-end px-3 pt-6 pb-[calc(10px+env(safe-area-inset-bottom,0px))] pointer-events-none')
const pillVariants = cva('mx-auto flex w-[50%] items-center gap-2 rounded-full glass bg-glass1 p-2 pointer-events-auto')

export function TabBar(props: {
  boardsActive: boolean
  settingsActive: boolean
  onGoHome: () => void
  onOpenSettings: () => void
}) {
  const { t } = useI18n()
  return (
    <nav data-tabs="" class={tabsVariants()}>
      <div class={pillVariants()}>
        <IconButton
          size="xl"
          variant={props.boardsActive ? 'glass' : undefined}
          onClick={props.onGoHome}
          aria-label={t('ui.boards')}
        >
          <BoardsIcon width={24} height={24} />
        </IconButton>
        <div class="flex-1" />
        <IconButton
          size="xl"
          variant={props.settingsActive ? 'glass' : undefined}
          onClick={props.onOpenSettings}
          aria-label={t('ui.settings')}
        >
          <SettingsIcon width={24} height={24} />
        </IconButton>
      </div>
    </nav>
  )
}
