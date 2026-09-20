import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { A } from '@solidjs/router'
import { ForumPage } from '../components/ForumPage'
import { PageHeader } from '../components/PageHeader'
import { TabBar } from '../components/TabBar'
import { ChevronRightIcon } from '../icons/ChevronRightIcon'
import type { MenuItem } from '../components/Menu/DropdownMenu'
import { useEntityActions } from '../entityActions'
import { useI18n } from '../i18n'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { useMobileTabs } from '../mobileNav'
import { boardPath } from '../route'
import type { BoardInfo, ForumInfo } from '../../core/views'

const pageVariants = cva('flex min-h-screen min-w-0 flex-col')
const screenVariants = cva('flex min-h-dvh flex-col bg-bg')
const chipsRowVariants = cva('flex items-center gap-2 overflow-x-auto px-4 pb-2.5 pt-0.5 [scrollbar-width:none]')
const chipVariants = cva('flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-md px-2.5 py-2 text-sm', {
  variants: {
    active: {
      true: 'bg-accent text-[#101d29]',
      false: 'bg-panel2 text-secondary',
    },
  },
  defaultVariants: { active: false },
})
const chipLogoVariants = cva('inline-flex h-logo-mobile items-center')
const contentPadVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-[calc(var(--mobile-tab)+18px)] pt-2.5')
const listVariants = cva('overflow-hidden rounded-lg bg-panel1')
const rowVariants = cva('mx-4 grid min-h-row cursor-pointer grid-cols-[64px_1fr_auto_auto] items-center gap-2 border-b border-separator py-2.5 text-fg no-underline last:border-b-0')
const slugVariants = cva('text-xl font-semibold text-accent leading-none')
const nameVariants = cva('text-xl text-fg leading-none')
const countVariants = cva('text-sm text-secondary leading-none')
const chevronVariants = cva('text-3xl text-secondary leading-none')

export function BoardsPage(props: {
  forumId: string
  boards: BoardInfo[]
  forums: ForumInfo[]
  onSelectForum: (forum: ForumInfo) => void
}) {
  const { t } = useI18n()
  const ea = useEntityActions()
  const mobile = useIsMobile()
  const tabs = useMobileTabs()
  const forumMenu = (): MenuItem[] => [
    { label: t('mod.debug'), onClick: () => ea.openLogs('forum') },
    { label: t('mod.bans'), onClick: () => ea.openBans(t('mod.bans'), props.forumId.replace(/^0x/, '')) },
    { label: t('mod.moderators'), onClick: () => ea.openForumModerators() },
  ]
  const totalPosts = () => props.boards.reduce((sum, board) => sum + board.count, 0)
  const forumPage = () => <ForumPage boardsCount={props.boards.length} totalPosts={totalPosts()} menu={forumMenu()} />
  return (
    <Show when={mobile()} fallback={<section class={pageVariants()}>{forumPage()}</section>}>
      <section class={screenVariants()}>
        <PageHeader title={t('ui.boards')} />
        <div class={chipsRowVariants()}>
          <For each={props.forums}>
            {(forum) => (
              <button
                type="button"
                class={chipVariants({ active: forum.forumId === props.forumId })}
                onClick={() => props.onSelectForum(forum)}
              >
                <span class={chipLogoVariants()}>
                  <img class="h-full w-auto" src="/static/logo.svg" alt="" />
                </span>
                {forum.host}
              </button>
            )}
          </For>
        </div>
        <div class={contentPadVariants()}>
          {forumPage()}
          <div class={listVariants()}>
            <For each={props.boards}>
              {(board) => (
                <A class={rowVariants()} href={boardPath(props.forumId, board.slug)}>
                  <div class={slugVariants()}>/{board.slug}/</div>
                  <div class={nameVariants()}>{board.title}</div>
                  <div class={countVariants()}>{board.count.toLocaleString()}</div>
                  <div class={chevronVariants()}>
                    <ChevronRightIcon width={24} height={24} />
                  </div>
                </A>
              )}
            </For>
          </div>
        </div>
        <TabBar boardsActive settingsActive={false} onGoHome={tabs.goHome} onOpenSettings={tabs.openSettings} />
      </section>
    </Show>
  )
}
