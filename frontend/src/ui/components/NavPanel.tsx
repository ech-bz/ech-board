import { createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { boardPath, settingKeyToSlug, type SettingSlug } from '../route'
import type { Section } from '../nav'
import { CheckIcon } from '../icons/CheckIcon'
import { ChevronDownIcon } from '../icons/ChevronDownIcon'
import { KeyRoundIcon } from '../icons/KeyRoundIcon'
import { PaletteIcon } from '../icons/PaletteIcon'
import { PanelLeftCloseIcon } from '../icons/PanelLeftCloseIcon'
import { PanelLeftOpenIcon } from '../icons/PanelLeftOpenIcon'
import { BoardsIcon } from '../icons/BoardsIcon'
import { SettingsIcon } from '../icons/SettingsIcon'
import { Capsule } from './Capsule'
import { IconButton } from './IconButton'
import { settingLabel } from '../settings'
import type { BoardInfo, ForumInfo } from '../../core/views'

const navScrollVariants = cva('flex-1 min-h-0 overflow-y-auto')
const navInnerVariants = cva('flex min-h-full flex-col pb-3')
const navPanelHeaderVariants = cva('glass-fade sticky top-0 z-header relative flex items-center px-4 py-3 min-h-header collapsed:justify-center gap-2.5')
const navPanelCollapseBtnVariants = cva('ml-auto collapsed:ml-0 hover:bg-hover')
const navSectionVariants = cva('min-h-0', {
  variants: {
    active: {
      true: 'block collapsed:flex collapsed:flex-col collapsed:items-center',
      false: 'hidden',
    },
  },
})
const navH2Variants = cva('w-full min-w-0 ellipsis text-center text-xl font-semibold leading-none')
const forumSelectWrapVariants = cva('relative mx-3 mb-3 collapsed:hidden')
const forumSelectBtnVariants = cva('flex w-full cursor-pointer items-center gap-2 rounded-md bg-panel2 px-2.5 py-2.5 text-left text-fg')
const logoVariants = cva('inline-flex h-logo items-center')
const logoImgVariants = cva('h-full w-auto')
const strongVariants = cva('font-semibold')
const subVariants = cva('w-full min-w-0 ellipsis text-center text-sm text-secondary leading-none')
const chevronVariants = cva('text-secondary')
const forumMenuVariants = cva('absolute left-0 right-0 top-[calc(100%+6px)] z-20 rounded-lg bg-active p-1.5', {
  variants: {
    open: {
      true: 'block',
      false: 'hidden',
    },
  },
})
const forumMenuItemVariants = cva('grid w-full cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-2.5 rounded-md bg-transparent px-2 py-2 text-left text-fg hover:bg-hover')
const forumItemSubVariants = cva('mt-0.5 text-sm text-secondary')
const forumItemCheckVariants = cva('text-lg text-accent')
const sectionLabelVariants = cva('px-4 pb-1.5 pt-2.5 text-sm uppercase tracking-wider text-secondary collapsed:hidden')
const navListVariants = cva('px-2 collapsed:flex collapsed:w-full collapsed:flex-col collapsed:items-center collapsed:px-1')
const navItemVariants = cva('grid cursor-pointer grid-cols-[44px_1fr_auto] items-center gap-1.5 px-2 py-2.5 rounded-md collapsed:flex collapsed:w-full collapsed:justify-center collapsed:py-2.5 hover:bg-hover', {
  variants: {
    active: {
      true: 'bg-active',
    },
  },
})
const navItemIconVariants = cva('font-bold text-accent collapsed:whitespace-nowrap')
const navItemLabelVariants = cva('font-semibold text-fg collapsed:hidden')
const navItemMetaVariants = cva('text-sm text-secondary collapsed:hidden')
const globalBarCapsuleVariants = cva('sticky bottom-3 z-header mt-auto gap-2 p-2 flex items-center rounded-full glass bg-glass1', {
  variants: {
    collapsed: {
      true: 'flex-col mx-auto w-fit',
      false: 'mx-3',
    },
  },
})
const spacerVariants = cva('flex-1')

export type { Section }

const SECURITY_SECTIONS: string[] = ['Master Key']
const APP_SECTIONS: string[] = ['Appearance']

export function NavPanel(props: {
  collapsed: boolean
  section: Section
  forums: ForumInfo[]
  forum: ForumInfo | null
  boards: BoardInfo[]
  board: BoardInfo | null
  setting: SettingSlug | null
  onToggleCollapsed: () => void
  onSelectForum: (forum: ForumInfo) => void
  onSelectBoard: (board: BoardInfo) => void
  onOpenSetting: (setting: SettingSlug) => void
  settingsHref: string
  onNavigateSection: (section: Section) => void
}) {
  const { t } = useI18n()
  const [forumMenuOpen, setForumMenuOpen] = createSignal(false)
  const isActiveSetting = (name: string) => settingKeyToSlug(name) === props.setting
  const title = () => (props.section === 'boards' ? t('ui.boards') : t('ui.settings'))
  return (
    <div class={navScrollVariants()}>
      <div class={navInnerVariants()}>
        <div class={navPanelHeaderVariants()}>
          <Capsule class="px-3 w-full collapsed:hidden">
            <h2 class={navH2Variants()}>{title()}</h2>
          </Capsule>
          <IconButton
            size="lg"
            variant="glass"
            class={navPanelCollapseBtnVariants()}
            aria-label={props.collapsed ? t('ui.expand') : t('ui.collapse')}
            onClick={props.onToggleCollapsed}
          >
            <Show when={props.collapsed} fallback={<PanelLeftCloseIcon width={18} height={18} />}>
              <PanelLeftOpenIcon width={18} height={18} />
            </Show>
          </IconButton>
        </div>
        <section class={navSectionVariants({ active: props.section === 'boards' })}>
          <div class={forumSelectWrapVariants()}>
            <button
              class={forumSelectBtnVariants()}
              onClick={(e) => {
                e.stopPropagation()
                setForumMenuOpen(!forumMenuOpen())
              }}
            >
              <div class={logoVariants()}>
                <img class={logoImgVariants()} src="/static/logo.svg" alt="" />
              </div>
              <div style={{ flex: 1 }}>
                <div class={strongVariants()}>{props.forum ? props.forum.host : ''}</div>
                <div class={subVariants()}>{t('ui.currentForum')}</div>
              </div>
              <div class={chevronVariants()}>
                <ChevronDownIcon width={18} height={18} />
              </div>
            </button>
            <div class={forumMenuVariants({ open: forumMenuOpen() })} onClick={(e) => e.stopPropagation()}>
              <For each={props.forums}>
                {(forum) => (
                  <button class={forumMenuItemVariants()} onClick={() => props.onSelectForum(forum)}>
                    <div class={logoVariants()}>
                      <img class={logoImgVariants()} src="/static/logo.svg" alt="" />
                    </div>
                    <div>
                      <div class={strongVariants()}>{forum.host}</div>
                      <div class={forumItemSubVariants()}>{forum.forumId.slice(0, 8)}</div>
                    </div>
                    <div class={forumItemCheckVariants()}>
                      <Show when={props.forum && forum.forumId === props.forum.forumId}>
                        <CheckIcon width={16} height={16} />
                      </Show>
                    </div>
                  </button>
                )}
              </For>
            </div>
          </div>
          <div class={sectionLabelVariants()}>{t('ui.boards')}</div>
          <div class={navListVariants()}>
            <For each={props.boards}>
              {(board) => (
                <a
                  class={navItemVariants({ active: props.board && board.uid === props.board.uid })}
                  title={board.title}
                  href={boardPath(props.forum ? props.forum.forumId : '', board.slug)}
                  onClick={(e) => {
                    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
                    e.preventDefault()
                    props.onSelectBoard(board)
                  }}
                >
                  <div class={navItemIconVariants()}>/{board.slug}/</div>
                  <div class={navItemLabelVariants()}>{board.title}</div>
                  <div class={navItemMetaVariants()}>{board.count.toLocaleString()}</div>
                </a>
              )}
            </For>
          </div>
        </section>
        <section class={navSectionVariants({ active: props.section === 'settings' })}>
          <div class={sectionLabelVariants()}>{t('ui.security')}</div>
          <div class={navListVariants()}>
            <For each={SECURITY_SECTIONS}>
              {(name) => (
                <a
                  class={navItemVariants({ active: isActiveSetting(name) })}
                  href={`${props.settingsHref}/${settingKeyToSlug(name)}`}
                  onClick={(e) => {
                    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
                    const slug = settingKeyToSlug(name)
                    if (!slug) return
                    e.preventDefault()
                    props.onOpenSetting(slug)
                  }}
                >
                  <div class={navItemIconVariants()}>
                    <KeyRoundIcon width={18} height={18} />
                  </div>
                  <div class={navItemLabelVariants()}>{settingLabel(name, t)}</div>
                </a>
              )}
            </For>
          </div>
          <div class={sectionLabelVariants()}>{t('ui.application')}</div>
          <div class={navListVariants()}>
            <For each={APP_SECTIONS}>
              {(name) => (
                <a
                  class={navItemVariants({ active: isActiveSetting(name) })}
                  href={`${props.settingsHref}/${settingKeyToSlug(name)}`}
                  onClick={(e) => {
                    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
                    const slug = settingKeyToSlug(name)
                    if (!slug) return
                    e.preventDefault()
                    props.onOpenSetting(slug)
                  }}
                >
                  <div class={navItemIconVariants()}>
                    <PaletteIcon width={18} height={18} />
                  </div>
                  <div class={navItemLabelVariants()}>{settingLabel(name, t)}</div>
                </a>
              )}
            </For>
          </div>
        </section>
        <div class={globalBarCapsuleVariants({ collapsed: props.collapsed })}>
          <IconButton
            size="xl"
            variant={props.section === 'boards' ? 'glass' : undefined}
            onClick={() => props.onNavigateSection('boards')}
          >
            <BoardsIcon width={24} height={24} />
          </IconButton>
          <div class={spacerVariants()} />
          <IconButton
            size="xl"
            variant={props.section === 'settings' ? 'glass' : undefined}
            onClick={() => props.onNavigateSection('settings')}
          >
            <SettingsIcon width={24} height={24} />
          </IconButton>
        </div>
      </div>
    </div>
  )
}
