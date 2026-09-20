import { createEffect, For, onCleanup, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ThreadPreview } from '../components/ThreadPreview'
import { IconButton } from '../components/IconButton'
import { PageHeader } from '../components/PageHeader'
import { TabBar } from '../components/TabBar'
import { SquarePenIcon } from '../icons/SquarePenIcon'
import { DropdownMenu, type MenuItem } from '../components/Menu/DropdownMenu'
import { useBoardInfo, useLoadingMore, useRows, useStore } from '../store'
import { useI18n } from '../i18n'
import { useEntityActions } from '../entityActions'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { useMobileTabs } from '../mobileNav'
import { canBoardSettings } from '../../core/intent/capabilities'
import { useNav } from '../nav'

const pageVariants = cva('flex min-h-screen min-w-0 flex-col')
const screenVariants = cva('flex min-h-dvh flex-col bg-bg')
const headerIconBtnVariants = cva('px-1 py-2 hover:bg-hover')
const boardMenuTriggerVariants = cva('px-1 py-2 hover:bg-hover')
const listVariants = cva('px-6 pb-12 pt-4')
const listMobileVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1.25rem+env(safe-area-inset-right,0px))] pb-[calc(var(--mobile-tab)+80px)] pt-2.5')
const sentinelVariants = cva('h-px')
const moreVariants = cva('py-3 text-center text-md text-secondary')

export function BoardPage(props: { forumId: string; slug: string; heading: string; host?: string }) {
  const { t, showDeleted } = useI18n()
  const mobile = useIsMobile()
  const tabs = useMobileTabs()
  const rows = useRows(showDeleted)
  const info = useBoardInfo()
  const loadingMore = useLoadingMore()
  const ea = useEntityActions()
  const store = useStore()
  const nav = useNav()
  const compose = () => nav.navigate({ page: 'compose', forumId: props.forumId, slug: props.slug })
  let sentinel: HTMLDivElement | undefined
  createEffect(() => {
    const board = info()
    if (!board || board.nextCursor === null) return
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) void store.loadMoreThreads()
      },
      { rootMargin: '300px' },
    )
    observer.observe(sentinel!)
    onCleanup(() => observer.disconnect())
  })
  const boardMenu = () => {
    const items: MenuItem[] = []
    const board = info()
    if (!board) return items
    items.push({ label: t('mod.debug'), onClick: () => ea.openLogs('board', store.boardObject() ?? undefined) })
    items.push({ label: t('mod.bans'), onClick: () => ea.openBans(t('mod.bans'), board.uid) })
    if (ea.canBanAny()) items.push({ label: t('mod.banUid'), onClick: () => ea.openBanUid() })
    items.push({ label: t('mod.moderators'), onClick: () => ea.openBoardModerators() })
    if (canBoardSettings(ea.roleKinds())) items.push({ label: t('mod.manageBoard'), onClick: () => ea.openBoardSettings() })
    return items
  }
  return (
    <section class={mobile() ? screenVariants() : pageVariants()}>
      <PageHeader
        title={props.heading}
        subtitle={props.host ?? ''}
        onBack={mobile() ? () => nav.back() : undefined}
        trailing={
          <>
            <IconButton size="lg" variant="glass" class={headerIconBtnVariants()} onClick={compose}>
              <SquarePenIcon width={20} height={20} />
            </IconButton>
            <DropdownMenu items={boardMenu()} triggerVariant="glass" class={boardMenuTriggerVariants()} />
          </>
        }
      />
      <div class={mobile() ? listMobileVariants() : listVariants()}>
        <For each={rows()}>
          {(row) => (
            <ThreadPreview
              row={row}
              forumId={props.forumId}
              slug={props.slug}
              onOpen={() => nav.navigate({ page: 'thread', forumId: props.forumId, slug: props.slug, threadNum: row.number })}
            />
          )}
        </For>
        <div ref={sentinel} class={sentinelVariants()} />
        <Show when={loadingMore()}>
          <div class={moreVariants()}>{t('common.loading')}</div>
        </Show>
      </div>
      <Show when={mobile()}>
        <TabBar boardsActive settingsActive={false} onGoHome={tabs.goHome} onOpenSettings={tabs.openSettings} />
      </Show>
    </section>
  )
}
