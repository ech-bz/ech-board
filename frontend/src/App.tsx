import { createEffect, createMemo, createSignal, onCleanup, Show } from 'solid-js'
import { Router, Route as RouterRoute, useLocation } from '@solidjs/router'
import { cva } from 'class-variance-authority'
import { ensureDefaultContext, loadBoardViewData, loadPostTooltip, loadThreadViewData } from './core/load'
import { getThreadUid } from './core/api/cache'
import { getCachedDefaultForumId } from './core/config'
import { loadBoards, listForums, type BoardInfo, type ForumInfo } from './core/views'
import { RealtimeClient } from './core/realtime/client'
import { ForumStore } from './store/store'
import { parseRoute, type Route } from './ui/route'
import type { RoleOption } from './core/intent/roles'
import { detectViewRoles } from './ui/roles'
import { StoreProvider } from './ui/store'
import { ModProvider } from './ui/mod'
import { ModalsProvider } from './ui/modals'
import { EntityActionsProvider } from './ui/entityActions'
import { ComposerBridgeProvider } from './ui/composerBridge'
import { GalleryProvider } from './ui/components/Gallery/GalleryProvider'
import { PostTooltipProvider } from './ui/postTooltip/PostTooltipProvider'
import { I18nProvider } from './ui/i18n'
import { TitleSync } from './ui/TitleSync'
import { reportError } from './ui/errors'
import { ThreadPage } from './ui/screens/ThreadPage'
import { BoardPage } from './ui/screens/BoardPage'
import { BoardsPage } from './ui/screens/BoardsPage'
import { ComposePage } from './ui/screens/ComposePage'
import { SettingsPage } from './ui/screens/SettingsPage'
import { NavPanel, type Section } from './ui/components/NavPanel'
import { IconSprite } from './ui/icons/IconSprite'
import { StatusPill } from './ui/components/StatusPill'
import { StateScreen } from './ui/components/StateScreen'
import { rememberContentPath, NavProvider, useNav } from './ui/nav'

const rootVariants = cva('min-h-dvh bg-bg')
const asideVariants = cva('max-[899px]:hidden fixed left-0 top-0 bottom-0 z-nav flex flex-col border-r border-separator bg-panel1', {
  variants: {
    collapsed: {
      true: 'collapsed w-nav-collapsed',
      false: 'w-nav',
    },
  },
})
const mainVariants = cva('min-h-dvh min-w-0 max-[899px]:ml-0', {
  variants: {
    offset: {
      nav: 'ml-nav',
      collapsed: 'ml-nav-collapsed',
    },
  },
  defaultVariants: { offset: 'nav' },
})

interface Context {
  forumId: string
  relayUrl: string
}

function contentPath(route: Route): string | null {
  if (route.page === 'board' || route.page === 'compose') return `b:${route.forumId ?? ''}:${route.slug}`
  if (route.page === 'thread') return `t:${route.forumId ?? ''}:${route.slug}:${route.threadNum}`
  return null
}

function Shell() {
  const store = new ForumStore()
  const location = useLocation()
  const nav = useNav()
  const [context, setContext] = createSignal<Context | null>(null)
  const [roles, setRoles] = createSignal<RoleOption[]>([])
  const [boards, setBoards] = createSignal<BoardInfo[]>([])
  const [boardsReady, setBoardsReady] = createSignal(false)
  const [forums, setForums] = createSignal<ForumInfo[]>([])
  const [collapsed, setCollapsed] = createSignal(localStorage.getItem('nav-collapsed') === '1')
  const [loading, setLoading] = createSignal(true)
  const [error, setError] = createSignal<string | null>(null)
  const [reloadKey, setReloadKey] = createSignal(0)
  const [reconnecting, setReconnecting] = createSignal(false)
  const [refreshing, setRefreshing] = createSignal(false)

  const realtime = new RealtimeClient({
    onReload: () => setReloadKey((key) => key + 1),
    onDrop: () => setReconnecting(true),
    onRestored: () => setReconnecting(false),
  })

  let generation = 0
  let lastReloadKey = -1
  let prevRoute: Route | null = null

  const run = async (route: Route, ctx: Context, refresh: boolean) => {
    const mine = ++generation
    let began = false
    if (refresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      if (route.page === 'thread') {
        const board = boards().find((item) => item.slug === route.slug)
        if (!board) {
          nav.navigate({ page: 'boards', forumId: ctx.forumId })
          return
        }
        const { threadUid } = await getThreadUid(ctx.relayUrl, route.slug, route.threadNum)
        realtime.begin(ctx.relayUrl, `thread/${threadUid}`)
        began = true
        const data = await loadThreadViewData(ctx.relayUrl, board.uid, threadUid)
        if (mine !== generation) return
        const kinds = detectViewRoles(data)
        store.setRoleKinds(kinds.map((role) => role.kind))
        setRoles(kinds)
        store.loadThread(data)
        return
      }
      if (route.page === 'board') {
        const board = boards().find((item) => item.slug === route.slug)
        if (!board) {
          nav.navigate({ page: 'boards', forumId: ctx.forumId })
          return
        }
        realtime.begin(ctx.relayUrl, `board/${board.uid}`)
        began = true
        const data = await loadBoardViewData(ctx.relayUrl, board.uid)
        if (mine !== generation) return
        const kinds = detectViewRoles(data)
        store.setRoleKinds(kinds.map((role) => role.kind))
        setRoles(kinds)
        store.loadBoard(data)
        return
      }
      if (route.page === 'compose') {
        const board = boards().find((item) => item.slug === route.slug)
        if (!board) {
          nav.navigate({ page: 'boards', forumId: ctx.forumId })
          return
        }
        const data = await loadBoardViewData(ctx.relayUrl, board.uid)
        if (mine !== generation) return
        const kinds = detectViewRoles(data)
        store.setRoleKinds(kinds.map((role) => role.kind))
        setRoles(kinds)
        store.loadBoard(data)
      }
    } catch (e) {
      if (mine === generation) setError(String(e))
    } finally {
      if (mine === generation) {
        if (began) realtime.commit((msg) => store.applyBatch(msg))
        setLoading(false)
        setRefreshing(false)
      }
    }
  }

  createEffect(() => {
    const key = reloadKey()
    const ctx = context()
    if (!ctx) return
    const route = parseRoute(location.pathname + location.hash, getCachedDefaultForumId())
    if (route.page === 'settings') {
      setLoading(false)
      return
    }
    if (route.page !== 'boards' && !boardsReady()) return
    const path = contentPath(route)
    if (path !== null) rememberContentPath(location.pathname + location.hash, ctx.forumId)
    const forced = key !== lastReloadKey && (route.page === 'board' || route.page === 'thread')
    lastReloadKey = key
    const prev = prevRoute
    prevRoute = route
    const sameContent = prev !== null && path !== null && contentPath(prev) === path
    const loaded = route.page === 'thread' ? store.page() !== null : route.page === 'board' ? store.boardInfo() !== null : false
    if (sameContent && !forced && loaded) return
    const refresh =
      prev !== null &&
      prev.page !== 'settings' &&
      prev.forumId === route.forumId &&
      ((sameContent && loaded) || (route.page === 'board' && prev.page === 'thread' && prev.slug === route.slug && store.boardInfo() !== null))
    void run(route, ctx, refresh)
  })

  createEffect(() => {
    ensureDefaultContext()
      .then((defaults) => setContext(defaults))
      .catch((e) => {
        setError(String(e))
        setLoading(false)
      })
  })

  createEffect(() => {
    const ctx = context()
    if (!ctx) return
    let cancelled = false
    setBoardsReady(false)
    setBoards([])
    loadBoards(ctx.relayUrl)
      .then((result) => {
        if (cancelled) return
        setBoards(result.boards)
        setForums(listForums())
        setBoardsReady(true)
      })
      .catch((e) => {
        if (cancelled) return
        setError(String(e))
        setLoading(false)
      })
    onCleanup(() => {
      cancelled = true
    })
  })

  onCleanup(() => {
    realtime.close()
    store.dispose()
  })

  const route = () => parseRoute(location.pathname + location.hash, getCachedDefaultForumId())
  const threadRoute = createMemo(() => {
    const current = route()
    return current.page === 'thread' ? current : null
  })
  const boardRoute = createMemo(() => {
    const current = route()
    return current.page === 'board' ? current : null
  })
  const composeRoute = createMemo(() => {
    const current = route()
    return current.page === 'compose' ? current : null
  })
  const section = (): Section => (route().page === 'settings' ? 'settings' : 'boards')
  const setting = () => {
    const current = route()
    return current.page === 'settings' ? (current.setting ?? null) : null
  }
  const currentBoard = () => {
    const current = route()
    if (current.page === 'board' || current.page === 'thread' || current.page === 'compose') {
      return boards().find((board) => board.slug === current.slug) ?? null
    }
    return null
  }
  const currentForum = () => forums().find((forum) => forum.forumId === context()?.forumId) ?? null
  const selectForum = (forum: ForumInfo) => {
    setContext({ forumId: forum.forumId, relayUrl: forum.relayUrl })
    nav.navigate({ page: 'boards', forumId: forum.forumId })
  }
  const goSection = (next: Section) => nav.section(next)
  const boardHeading = () => {
    const board = currentBoard()
    if (!board) return ''
    return board.title ? `/${board.slug}/ - ${board.title}` : `/${board.slug}/`
  }

  createEffect(() => {
    localStorage.setItem('nav-collapsed', collapsed() ? '1' : '0')
  })

  let lastKey: string | null = null
  const savedScrolls = new Map<string, number>()
  const contentKey = () => {
    const current = route()
    if (current.page === 'thread') return `thread:${current.slug}:${current.threadNum}`
    if (current.page === 'board') return `board:${current.slug}`
    if (current.page === 'compose') return `compose:${current.slug}`
    if (current.page === 'settings') return `settings:${current.setting ?? ''}`
    return 'boards'
  }
  createEffect(() => {
    const key = contentKey()
    if (lastKey != null && lastKey !== key) savedScrolls.set(lastKey, window.scrollY)
    lastKey = key
    window.scrollTo(0, savedScrolls.get(key) ?? 0)
  })

  const panel = () => (
    <NavPanel
      collapsed={collapsed()}
      section={section()}
      forums={forums()}
      forum={currentForum()}
      boards={boards()}
      board={currentBoard()}
      setting={setting()}
      onToggleCollapsed={() => setCollapsed(!collapsed())}
      onSelectForum={selectForum}
      onSelectBoard={(board) => nav.navigate({ page: 'board', forumId: context()!.forumId, slug: board.slug })}
      onOpenSetting={(slug) => nav.navigate({ page: 'settings', setting: slug })}
      settingsHref="/settings"
      onNavigateSection={goSection}
    />
  )

  return (
    <I18nProvider>
      <StoreProvider store={store}>
        <ModProvider roles={roles()} onError={reportError}>
          <ModalsProvider roles={roles()} relayUrl={context()?.relayUrl ?? ''}>
            <EntityActionsProvider roles={roles()}>
            <ComposerBridgeProvider>
              <GalleryProvider>
                <PostTooltipProvider
                  resolve={(uid) => {
                    const current = route()
                    const slug = current && 'slug' in current ? current.slug : ''
                    const local = slug ? store.tooltipPost(uid, slug) : null
                    if (local) return Promise.resolve(local)
                    const info = store.info()
                    return info ? loadPostTooltip(info.relayUrl, info.forumId, uid) : Promise.resolve(null)
                  }}
                >
                  <div class={rootVariants()}>
                    <IconSprite />
                    <TitleSync boards={boards()} />
                    <aside class={asideVariants({ collapsed: collapsed() })}>{panel()}</aside>
                    <main class={mainVariants({ offset: collapsed() ? 'collapsed' : 'nav' })}>
                      <Show
                        when={context() && !loading()}
                        fallback={<StateScreen error={error()} />}
                      >
                        <Show when={!error()} fallback={<StateScreen error={error()} />}>
                          <Show when={threadRoute()}>
                            {(r) => <ThreadPage forumId={r().forumId ?? ''} slug={r().slug} threadNum={r().threadNum} />}
                          </Show>
                          <Show when={boardRoute()}>
                            {(r) => (
                              <BoardPage
                                forumId={r().forumId ?? ''}
                                slug={r().slug}
                                heading={boardHeading()}
                                host={currentForum()?.host}
                              />
                            )}
                          </Show>
                          <Show when={composeRoute()}>
                            {(r) => <ComposePage forumId={r().forumId ?? ''} slug={r().slug} heading={boardHeading()} />}
                          </Show>
                          <Show when={route().page === 'boards' ? true : null}>
                            <BoardsPage
                              forumId={context()!.forumId}
                              boards={boards()}
                              forums={forums()}
                              onSelectForum={selectForum}
                            />
                          </Show>
                          <Show when={route().page === 'settings' ? true : null}>
                            <SettingsPage setting={setting()} baseHref="/settings" />
                          </Show>
                        </Show>
                      </Show>
                    </main>
                    <StatusPill reconnecting={reconnecting()} refreshing={refreshing()} />
                  </div>
                </PostTooltipProvider>
              </GalleryProvider>
            </ComposerBridgeProvider>
          </EntityActionsProvider>          </ModalsProvider>        </ModProvider>
      </StoreProvider>
    </I18nProvider>
  )
}

export default function App() {
  return (
    <Router>
      <RouterRoute path="*" component={() => (
        <NavProvider>
          <Shell />
        </NavProvider>
      )}
      />
    </Router>
  )
}
