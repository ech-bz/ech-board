import { createEffect, Match, onMount, Show, Switch } from 'solid-js'
import { cva } from 'class-variance-authority'
import { PageHeader } from '../components/PageHeader'
import { TabBar } from '../components/TabBar'
import { StateScreen } from '../components/StateScreen'
import { OverboardPanel } from '../components/mod/OverboardPanel'
import { ReportsPanel } from '../components/mod/ReportsPanel'
import { ActionsPanel } from '../components/mod/ActionsPanel'
import { GalleryPanel } from '../components/mod/GalleryPanel'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { useI18n } from '../i18n'
import { useNav } from '../nav'
import { useMobileTabs } from '../mobileNav'
import { modData } from '../mod/modData'
import { MOD_TABS, openModTabs } from '../modTabs'
import { accessLevel } from '../roles'
import type { ModTab } from '../route'
import type { RoleKind } from '../../core/intent/roles'

const screenVariants = cva('flex h-dvh min-w-0 flex-col')
const bodyVariants = cva('flex min-h-0 flex-1 flex-col', {
  variants: {
    mobile: {
      true: 'pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-[calc(5rem+max(16px,env(safe-area-inset-bottom,0px)))] pt-2',
      false: 'px-6 pb-6 pt-2',
    },
  },
})

export function ModPage(props: {
  tab: ModTab
  forumId: string
  boards: string[]
  ready: boolean
  canModerate: boolean
  roleKinds: RoleKind[]
}) {
  const { t } = useI18n()
  const nav = useNav()
  const mobile = useIsMobile()
  const tabs = useMobileTabs()
  createEffect(() => {
    if (props.ready) modData.seed(props.boards)
  })
  createEffect(() => modData.setCanModerate(props.canModerate))
  onMount(openModTabs)
  const level = () => (props.canModerate ? accessLevel(props.roleKinds) : 0)
  const title = () => t(MOD_TABS.find((item) => item.tab === props.tab)!.label)
  return (
    <Show when={props.ready} fallback={<StateScreen />}>
        <section class={screenVariants()}>
          <PageHeader title={title()} subtitle={t('modUi.accessLevel', { n: level() })} onBack={mobile() ? () => nav.back() : undefined} />
          <div class={bodyVariants({ mobile: mobile() })}>
            <Switch>
              <Match when={props.tab === 'overboard'}>
                <OverboardPanel forumId={props.forumId} roleKinds={props.roleKinds} canModerate={props.canModerate} />
              </Match>
              <Match when={props.tab === 'reports'}>
                <ReportsPanel forumId={props.forumId} canModerate={props.canModerate} />
              </Match>
              <Match when={props.tab === 'actions'}>
                <ActionsPanel forumId={props.forumId} canModerate={props.canModerate} />
              </Match>
              <Match when={props.tab === 'gallery'}>
                <GalleryPanel forumId={props.forumId} boards={props.boards} canModerate={props.canModerate} />
              </Match>
            </Switch>
          </div>
          <Show when={mobile()}>
            <TabBar boardsActive={false} settingsActive={false} onGoHome={tabs.goHome} onOpenSettings={tabs.openSettings} />
          </Show>
        </section>
    </Show>
  )
}
