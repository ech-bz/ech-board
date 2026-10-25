import { For, Show } from 'solid-js'
import { useLocation } from '@solidjs/router'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { useNav } from '../nav'
import { getCachedDefaultForumId } from '../../core/config'
import { parseRoute } from '../route'
import { MOD_TABS, closeModTabs, modTabsOpen, openModTabs } from '../modTabs'
import { BoardsIcon } from '../icons/BoardsIcon'
import { SettingsIcon } from '../icons/SettingsIcon'
import { ShieldIcon } from '../icons/ShieldIcon'
import { ChevronLeftIcon } from '../icons/ChevronLeftIcon'
import { IconButton } from './IconButton'

const tabsVariants = cva('fixed inset-x-0 bottom-0 z-tabbar flex items-end px-3 pt-1 pb-[calc(0.5rem+env(safe-area-inset-bottom,0px))] pointer-events-none')
const pillVariants = cva('mx-auto flex h-[60px] w-72 flex-none items-center rounded-full glass bg-glass1 p-2 pointer-events-auto')
const rowVariants = cva('fade-in flex w-full items-center justify-between')

export function TabBar(props: {
  boardsActive: boolean
  settingsActive: boolean
  onGoHome: () => void
  onOpenSettings: () => void
}) {
  const { t } = useI18n()
  const nav = useNav()
  const location = useLocation()
  const modTab = () => {
    const route = parseRoute(location.pathname, getCachedDefaultForumId())
    return route.page === 'mod' ? route.tab : null
  }
  return (
    <nav data-tabs="" class={tabsVariants()}>
      <div class={pillVariants()}>
        <Show
          when={modTabsOpen()}
          fallback={
            <div class={rowVariants()}>
              <IconButton
                size="lg"
                variant={props.boardsActive ? 'glass' : undefined}
                onClick={props.onGoHome}
                aria-label={t('ui.boards')}
              >
                <BoardsIcon width={24} height={24} />
              </IconButton>
              <IconButton
                size="lg"
                variant={props.settingsActive ? 'glass' : undefined}
                onClick={props.onOpenSettings}
                aria-label={t('ui.settings')}
              >
                <SettingsIcon width={24} height={24} />
              </IconButton>
              <IconButton size="lg" onClick={openModTabs} aria-label={t('ui.moderation')}>
                <ShieldIcon width={24} height={24} />
              </IconButton>
            </div>
          }
        >
          <div class={rowVariants()}>
            <For each={MOD_TABS}>
              {(item) => (
                <IconButton
                  size="lg"
                  variant={modTab() === item.tab ? 'glass' : undefined}
                  onClick={() => nav.navigate({ page: 'mod', tab: item.tab })}
                  aria-label={t(item.label)}
                >
                  <item.Icon width={24} height={24} />
                </IconButton>
              )}
            </For>
            <IconButton size="lg" onClick={closeModTabs} aria-label={t('ui.back')}>
              <ChevronLeftIcon width={24} height={24} />
            </IconButton>
          </div>
        </Show>
      </div>
    </nav>
  )
}
