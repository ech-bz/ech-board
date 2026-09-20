import { Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { A } from '@solidjs/router'
import { useI18n } from '../i18n'
import { AppearanceView } from '../components/AppearanceView'
import { MasterKeyView } from '../components/MasterKeyView'
import { PageHeader } from '../components/PageHeader'
import { TabBar } from '../components/TabBar'
import { KeyRoundIcon } from '../icons/KeyRoundIcon'
import { PaletteIcon } from '../icons/PaletteIcon'
import { ChevronRightIcon } from '../icons/ChevronRightIcon'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { useNav } from '../nav'
import { useMobileTabs } from '../mobileNav'
import { settingKeyToSlug, type SettingSlug } from '../route'
import { settingLabel } from '../settings'

const viewSectionVariants = cva('flex min-h-screen min-w-0 flex-col')
const stateWrapVariants = cva('flex min-h-state items-center justify-center p-12 text-center')
const stateBoxVariants = cva('max-w-narrow')
const stateTitleVariants = cva('mb-1.5 text-xl font-semibold text-fg leading-none')
const stateBodyVariants = cva('leading-normal')
const screenVariants = cva('flex min-h-dvh flex-col bg-bg')
const mobilePadVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-[calc(var(--mobile-tab)+18px)] pt-2.5')
const mobileLabelVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-1.5 pt-2.5 text-sm uppercase tracking-wider text-secondary')
const mobileListVariants = cva('overflow-hidden rounded-lg')
const mobileRowVariants = cva('ml-4 grid min-h-row cursor-pointer grid-cols-[52px_1fr_auto_auto] items-center gap-2 border-b border-separator py-2.5 pr-0 text-fg no-underline last:border-b-0')
const mobileRowIconVariants = cva('text-xl font-semibold text-accent leading-none')
const mobileRowTitleVariants = cva('text-xl leading-none')
const mobileRowChevronVariants = cva('text-3xl text-secondary leading-none')

const SETTING_KEYS: Record<SettingSlug, string> = { key: 'Master Key', appearance: 'Appearance' }

export function SettingsPage(props: { setting: SettingSlug | null; baseHref: string }) {
  const { t } = useI18n()
  const nav = useNav()
  const mobile = useIsMobile()
  const tabs = useMobileTabs()
  const key = () => (props.setting ? SETTING_KEYS[props.setting] : null)
  const label = () => (key() ? settingLabel(key()!, t) : t('ui.settings'))
  const view = () => (
    <Show when={key() === 'Master Key'} fallback={<AppearanceView />}>
      <MasterKeyView />
    </Show>
  )
  return (
    <Show
      when={mobile()}
      fallback={
        <Show
          when={key()}
          fallback={
            <section class={viewSectionVariants()}>
              <div class={stateWrapVariants()}>
                <div class={stateBoxVariants()}>
                  <div class={stateTitleVariants()}>{t('ui.settingsSelectTitle')}</div>
                  <div class={stateBodyVariants()}>{t('ui.settingsSelectText')}</div>
                </div>
              </div>
            </section>
          }
        >
          <section class={viewSectionVariants()}>
            <PageHeader title={label()} />
            {view()}
          </section>
        </Show>
      }
    >
      <section class={screenVariants()}>
        <PageHeader title={label()} onBack={props.setting ? () => nav.navigate({ page: 'settings' }) : undefined} />
        <Show when={key()} fallback={<SettingsList baseHref={props.baseHref} />}>
          <div class={mobilePadVariants()}>{view()}</div>
        </Show>
        <TabBar boardsActive={false} settingsActive onGoHome={tabs.goHome} onOpenSettings={tabs.openSettings} />
      </section>
    </Show>
  )
}

function SettingsList(props: { baseHref: string }) {
  const { t } = useI18n()
  return (
    <div class={mobilePadVariants()}>
      <div class={mobileLabelVariants()}>{t('ui.security')}</div>
      <div class={mobileListVariants()}>
        <SettingsLink name="Master Key" baseHref={props.baseHref} />
      </div>
      <div class={mobileLabelVariants()}>{t('ui.application')}</div>
      <div class={mobileListVariants()}>
        <SettingsLink name="Appearance" baseHref={props.baseHref} />
      </div>
    </div>
  )
}

function SettingsLink(props: { name: string; baseHref: string }) {
  const { t } = useI18n()
  const nav = useNav()
  return (
    <A
      class={mobileRowVariants()}
      href={`${props.baseHref}/${settingKeyToSlug(props.name)}`}
      onClick={(e) => {
        if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
        const slug = settingKeyToSlug(props.name)
        if (!slug) return
        e.preventDefault()
        nav.navigate({ page: 'settings', setting: slug })
      }}
    >
      <div class={mobileRowIconVariants()}>
        <Show when={props.name === 'Master Key'} fallback={<PaletteIcon width={18} height={18} />}>
          <KeyRoundIcon width={18} height={18} />
        </Show>
      </div>
      <div class={mobileRowTitleVariants()}>{settingLabel(props.name, t)}</div>
      <div />
      <div class={mobileRowChevronVariants()}>
        <ChevronRightIcon width={22} height={22} />
      </div>
    </A>
  )
}
