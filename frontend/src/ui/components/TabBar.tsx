import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { BoardsIcon } from '../icons/BoardsIcon'
import { SettingsIcon } from '../icons/SettingsIcon'
import { PencilIcon } from '../icons/PencilIcon'
import { IconButton } from './IconButton'

const tabsVariants = cva('fixed inset-x-0 bottom-0 z-tabbar flex items-end px-3 pt-1 pb-[calc(0.5rem+env(safe-area-inset-bottom,0px))] pointer-events-none')
const pillVariants = cva('mx-auto flex h-[60px] w-72 flex-none items-center rounded-full glass bg-glass1 p-2 pointer-events-auto')

export function TabBar(props: {
  boardsActive: boolean
  settingsActive: boolean
  onGoHome: () => void
  onOpenSettings: () => void
  onOpenComposer?: () => void
}) {
  const { t } = useI18n()
  return (
    <nav data-tabs="" class={tabsVariants()}>
      <div class={pillVariants()}>
        <div class="flex w-full items-center justify-between">
          <IconButton
            size="xl"
            variant={props.boardsActive ? 'glass' : undefined}
            onClick={props.onGoHome}
            aria-label={t('ui.boards')}
          >
            <BoardsIcon width={24} height={24} />
          </IconButton>
          {props.onOpenComposer && (
            <IconButton size="xl" onClick={() => props.onOpenComposer?.()} aria-label={t('postForm.newMessage')}>
              <PencilIcon width={24} height={24} />
            </IconButton>
          )}
          <IconButton
            size="xl"
            variant={props.settingsActive ? 'glass' : undefined}
            onClick={props.onOpenSettings}
            aria-label={t('ui.settings')}
          >
            <SettingsIcon width={24} height={24} />
          </IconButton>
        </div>
      </div>
    </nav>
  )
}
