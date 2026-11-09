import { createSignal, For, onCleanup, onMount, Show, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { logoFor } from '../vista/boardIcons'
import { useEntityActions } from '../entityActions'
import { useStore } from '../store'
import { PotatoHeadContext } from '../potatoHead'
import { boardPath, boardsPath, settingKeyToSlug, type Route, type SettingSlug } from '../route'
import { settingLabel } from '../settings'
import type { Section } from '../nav'
import type { BoardInfo, ForumInfo } from '../../core/views'

const barVariants = cva('flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-separator px-3 py-1.5 text-md')
const barLinkVariants = cva('cursor-pointer border-0 bg-transparent p-0 text-accent [font:inherit] hover:text-accent-hover hover:underline')
const forumSelectVariants = cva('max-w-full border border-panel3 bg-bg px-1 py-0.5 text-sm text-fg [font:inherit]')
const headVariants = cva('px-3 pb-3 pt-4 text-center')
const logoVariants = cva('mx-auto mb-3 block h-[88px] w-auto')
const columnsVariants = cva('flex items-start max-[899px]:flex-col')
const navVariants = cva('w-[260px] flex-none px-3 p-4 max-[899px]:w-full')
const navLabelVariants = cva('py-1 text-lg text-secondary')
const navListVariants = cva('max-[899px]:flex max-[899px]:flex-wrap max-[899px]:gap-x-3 pt-2')
const navItemVariants = cva('block py-1 pl-5 text-md text-accent no-underline [overflow-wrap:anywhere]', {
  variants: { active: { true: 'bg-active', false: '' } },
  defaultVariants: { active: false },
})
const navTitleVariants = cva('max-[899px]:hidden')
const mainVariants = cva('min-w-0 flex-1 pl-1 max-[899px]:w-full')

const SETTINGS: string[] = ['Master Key', 'Appearance']

export function PotatoLayout(props: {
  page: Route['page']
  forums: ForumInfo[]
  forum: ForumInfo | null
  boards: BoardInfo[]
  board: BoardInfo | null
  setting: SettingSlug | null
  minHeight: string
  onSection: (section: Section) => void
  onSelectForum: (forum: ForumInfo) => void
  onSelectBoard: (board: BoardInfo) => void
  onOpenSetting: (setting: SettingSlug) => void
  children: JSX.Element
}) {
  const { t, appearance, preset, centerPotatoThreads, centerPotatoBoards, potatoBoardsBar, potatoHeader } = useI18n()
  const ea = useEntityActions()
  const store = useStore()
  const [head, setHead] = createSignal<HTMLElement>()
  const plain = (e: MouseEvent) => e.button === 0 && !e.metaKey && !e.ctrlKey && !e.shiftKey
  const debug = () => {
    if (props.page === 'boards') {
      ea.openLogs('forum')
    } else if (props.page === 'board' || props.page === 'compose') {
      ea.openLogs('board', store.boardObject() ?? undefined)
    } else if (props.page === 'thread') {
      const uid = store.threadUid()
      ea.openLogs('thread', uid ? (store.threadObject(uid) ?? undefined) : undefined)
    }
  }
  const forumId = () => props.forum?.forumId ?? ''
  let layoutEl: HTMLDivElement | undefined
  // Отмечаем, сдвинулась ли страница и «прилип» ли блок досок: темы могут делать их прозрачными, пока они стоят на месте.
  onMount(() => {
    let raf = 0
    const update = () => {
      raf = 0
      const root = layoutEl
      if (!root) return
      const scrolled = window.scrollY > 0
      root.toggleAttribute('data-scrolled', scrolled)
      const nav = root.querySelector<HTMLElement>('[data-potato-nav]')
      if (nav) {
        const cs = getComputedStyle(nav)
        const top = Number.parseFloat(cs.top) || 0
        nav.toggleAttribute('data-stuck', scrolled && cs.position === 'sticky' && nav.getBoundingClientRect().top <= top + 0.5)
      }
    }
    const schedule = () => {
      if (!raf) raf = requestAnimationFrame(update)
    }
    window.addEventListener('scroll', schedule, { passive: true })
    window.addEventListener('resize', schedule)
    schedule()
    onCleanup(() => {
      window.removeEventListener('scroll', schedule)
      window.removeEventListener('resize', schedule)
      if (raf) cancelAnimationFrame(raf)
    })
  })
  return (
    <PotatoHeadContext.Provider value={head}>
      <div ref={layoutEl} data-potato-layout="" data-potato-header={potatoHeader()} data-potato-center-boards={centerPotatoBoards() && props.page !== 'settings' ? '' : undefined} data-potato-boards-bar={potatoBoardsBar()}>
        <header data-potato-topbar="" class={barVariants()}>
          <a
            class={barLinkVariants()}
            href={boardsPath(forumId())}
            onClick={(e) => {
              if (!plain(e)) return
              e.preventDefault()
              props.onSection('boards')
            }}
          >
            {t('ui.home')}
          </a>
          <a
            class={barLinkVariants()}
            href="/settings"
            onClick={(e) => {
              if (!plain(e)) return
              e.preventDefault()
              props.onSection('settings')
            }}
          >
            {t('ui.settings')}
          </a>
          <Show when={props.page !== 'settings'}>
            <button type="button" class={barLinkVariants()} onClick={debug}>
              {t('mod.debug')}
            </button>
          </Show>
          <Show when={props.forums.length > 0}>
            <select
              class={forumSelectVariants()}
              aria-label={t('ui.currentForum')}
              onChange={(e) => {
                const next = props.forums.find((item) => item.forumId === e.currentTarget.value)
                if (next) props.onSelectForum(next)
              }}
            >
              <For each={props.forums}>
                {(item) => (
                  <option value={item.forumId} selected={item.forumId === forumId()}>
                    {item.host}
                  </option>
                )}
              </For>
            </select>
          </Show>
        </header>
        <div ref={setHead} data-potato-head="" class={headVariants()}>
          <img class={logoVariants()} src={logoFor(appearance(), preset())} alt="" />
        </div>
        <div data-potato-columns="" class={columnsVariants()}>
          <nav data-potato-nav="" class={navVariants()}>
            <Show
              when={props.page === 'settings'}
              fallback={
                <>
                  <div class={navLabelVariants()}>{t('ui.boards')}</div>
                  <div class={navListVariants()}>
                    <For each={props.boards}>
                      {(board) => (
                        <a
                          class={navItemVariants({ active: !!props.board && board.uid === props.board.uid })}
                          title={`${board.title} (${board.count.toLocaleString()})`}
                          href={boardPath(forumId(), board.slug)}
                          onClick={(e) => {
                            if (!plain(e)) return
                            e.preventDefault()
                            props.onSelectBoard(board)
                          }}
                        >
                          {`/${board.slug}/`}
                          <span class={navTitleVariants()}>{` - ${board.title}`}</span>
                        </a>
                      )}
                    </For>
                  </div>
                </>
              }
            >
              <div class={navLabelVariants()}>{t('ui.settings')}</div>
              <div class={navListVariants()}>
                <For each={SETTINGS}>
                  {(name) => {
                    const slug = settingKeyToSlug(name)
                    return (
                      <a
                        class={navItemVariants({ active: slug === props.setting })}
                        href={`/settings/${slug}`}
                        onClick={(e) => {
                          if (!plain(e) || !slug) return
                          e.preventDefault()
                          props.onOpenSetting(slug)
                        }}
                      >
                        {settingLabel(name, t)}
                      </a>
                    )
                  }}
                </For>
              </div>
            </Show>
          </nav>
          <main data-potato-main="" data-potato-center-threads={centerPotatoThreads() ? '' : undefined} class={mainVariants()} style={{ 'min-height': props.minHeight }}>
            {props.children}
          </main>
        </div>
      </div>
    </PotatoHeadContext.Provider>
  )
}
