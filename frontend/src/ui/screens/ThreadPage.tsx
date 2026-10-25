import { createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useLocation } from '@solidjs/router'
import { Post } from '../components/Post'
import { PostComposer } from '../components/PostComposer/PostComposer'
import { ThreadNavDown, ThreadNavUp } from '../components/ThreadNav'
import { PageHeader } from '../components/PageHeader'
import { usePage, usePosts, useStore } from '../store'
import { useI18n } from '../i18n'
import { useEntityActions } from '../entityActions'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { PostSelectionProvider } from '../postSelection/PostSelectionProvider'
import { ThreadSelectionMenu } from '../postSelection/ThreadSelectionMenu'
import { threadModItems } from '../threadModItems'
import { createThreadUnread } from '../unread'
import { scrollToPost } from '../scroll'
import { useNav } from '../nav'
import { canBanByUid, canPostAction } from '../../core/intent/capabilities'

const pageVariants = cva('flex min-h-screen min-w-0 flex-col')
const screenVariants = cva('flex min-h-dvh flex-col')
const menuTriggerVariants = cva('ml-auto px-1 py-2 hover:bg-hover')
const listVariants = cva('pl-4 pr-3.5 pb-[4.25rem]')
const listMobileVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-[4.25rem] pt-2.5')
const markerVariants = cva('my-3 w-full border-t-2 border-dashed border-accent will-change-transform')

export function ThreadPage(props: { forumId: string; slug: string; threadNum: number }) {
  const { showDeleted, t } = useI18n()
  const [editingUid, setEditingUid] = createSignal<string | null>(null)
  const posts = usePosts(showDeleted)
  const page = usePage()
  const ea = useEntityActions()
  const canSelect = createMemo(() => canPostAction('delete', ea.roleKinds(), false, Number.MAX_SAFE_INTEGER))
  const threadMenu = () => {
    const current = page()
    if (!current) return []
    const items = threadModItems(t, ea, { threadUid: current.uid, closed: current.closed, deleted: current.deleted })
    items.push({ label: t('mod.moderators'), onClick: () => ea.openThreadModerators(current.uid) })
    items.push({ label: t('mod.bans'), onClick: () => ea.openBans(t('mod.bans'), current.uid) })
    if (ea.canBanAny() && canBanByUid(ea.roleKinds())) items.push({ label: t('mod.banUid'), onClick: () => ea.openBanUid() })
    return items
  }
  return (
    <PostSelectionProvider canSelect={canSelect()} uids={posts}>
      <ThreadBody
        forumId={props.forumId}
        slug={props.slug}
        threadNum={props.threadNum}
        posts={posts}
        threadMenu={threadMenu}
        editingUid={editingUid()}
        onEdit={setEditingUid}
        onEditDone={() => setEditingUid(null)}
      />
    </PostSelectionProvider>
  )
}

function ThreadBody(props: {
  forumId: string
  slug: string
  threadNum: number
  posts: () => readonly string[]
  threadMenu: () => ReturnType<typeof threadModItems>
  editingUid: string | null
  onEdit: (uid: string) => void
  onEditDone: () => void
}) {
  const { t } = useI18n()
  const store = useStore()
  const nav = useNav()
  const mobile = useIsMobile()
  const page = usePage()
  const location = useLocation()
  const posts = props.posts
  const heading = () => store.threadTitle() || `#${props.threadNum}`
  const sub = () => (store.threadTitle() ? `/${props.slug}/ - ${t('ui.threadLabel', { n: props.threadNum })}` : `/${props.slug}/`)
  const back = () => nav.back()
  const unread = createThreadUnread(
    () => posts().length,
    () => store.resyncSeq(),
  )
  let seenResync = store.resyncSeq()
  let atBottom = false
  let pinned = false
  let lastY = 0
  let lastMax = 0
  let adjustUntil = 0
  let followUntil = 0
  let prevLength = 0
  const [atTop, setAtTop] = createSignal(true)
  const [atEnd, setAtEnd] = createSignal(false)
  const bottomReached = () => window.scrollY >= lastMax - 8
  const measure = () => {
    const max = document.documentElement.scrollHeight - window.innerHeight
    const y = window.scrollY
    const now = performance.now()
    setAtTop(y <= 1)
    lastMax = max
    if (now < adjustUntil) {
      lastY = y
      return
    }
    if (max <= 0 || y >= max - 0.5) {
      atBottom = true
      pinned = true
      followUntil = 0
    } else if (now < followUntil) {
      followUntil = now + 300
    } else {
      atBottom = false
      pinned = pinned && y >= lastY - 0.5
    }
    setAtEnd(atBottom)
    lastY = y
  }
  const scrollKeys = new Set(['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '])
  const yieldScroll = (e: Event) => {
    if (e instanceof KeyboardEvent && !scrollKeys.has(e.key)) return
    followUntil = 0
  }
  window.addEventListener('scroll', measure, { passive: true })
  window.addEventListener('resize', measure)
  window.addEventListener('wheel', yieldScroll, { passive: true })
  window.addEventListener('touchstart', yieldScroll, { passive: true })
  window.addEventListener('keydown', yieldScroll)
  onCleanup(() => {
    window.removeEventListener('scroll', measure)
    window.removeEventListener('resize', measure)
    window.removeEventListener('wheel', yieldScroll)
    window.removeEventListener('touchstart', yieldScroll)
    window.removeEventListener('keydown', yieldScroll)
  })
  const mounted = requestAnimationFrame(measure)
  onCleanup(() => cancelAnimationFrame(mounted))

  const up = () => {
    adjustUntil = 0
    atBottom = false
    pinned = false
    setAtEnd(false)
    window.scrollTo(0, 0)
  }
  const down = () => {
    adjustUntil = 0
    atBottom = true
    pinned = true
    setAtEnd(true)
    window.scrollTo(0, document.documentElement.scrollHeight)
  }

  createEffect(() => {
    const length = posts().length
    const previous = prevLength
    prevLength = length
    if (previous === 0 || length <= previous) return
    const resync = store.resyncSeq()
    const missed = resync !== seenResync
    seenResync = resync
    if (missed || document.visibilityState !== 'visible') {
      atBottom = false
      pinned = false
      setAtEnd(false)
      lastMax = document.documentElement.scrollHeight - window.innerHeight
      lastY = window.scrollY
      return
    }
    if (!pinned && bottomReached()) pinned = true
    if (!pinned) return
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        followUntil = performance.now() + 1000
        window.scrollTo({ top: document.documentElement.scrollHeight, behavior: 'smooth' })
      }),
    )
  })

  createEffect(() => {
    const hash = location.hash.replace(/^#/, '')
    const num = Number.parseInt(hash, 10)
    if (!Number.isFinite(num)) return
    const raf = requestAnimationFrame(() => scrollToPost(num))
    onCleanup(() => cancelAnimationFrame(raf))
  })

  createEffect(() => {
    posts()
    const remeasure = requestAnimationFrame(measure)
    onCleanup(() => cancelAnimationFrame(remeasure))
    const composer = document.querySelector<HTMLElement>('[data-composer]')
    const threadList = document.querySelector<HTMLElement>('[data-thread-scroll]')
    if (!composer || !threadList) return
    let height = composer.getBoundingClientRect().height
    const syncComposerSpace = (composerHeight: number) => {
      const padding = `${Math.ceil(composerHeight + 8)}px`
      if (threadList.style.paddingBottom !== padding) threadList.style.paddingBottom = padding
    }
    syncComposerSpace(height)
    const observer = new ResizeObserver(() => {
      const wasAtBottom = lastMax > 0 && bottomReached()
      if (!pinned && wasAtBottom) pinned = true
      const next = composer.getBoundingClientRect().height
      const delta = next - height
      height = next
      syncComposerSpace(next)
      if (delta > 0 && pinned && followUntil === 0 && document.visibilityState === 'visible') {
        adjustUntil = performance.now() + 200
        window.scrollTo(0, document.documentElement.scrollHeight)
      }
    })
    observer.observe(composer)
    onCleanup(() => observer.disconnect())
  })

  return (
    <section class={mobile() ? screenVariants() : pageVariants()}>
      <PageHeader
        title={heading()}
        subtitle={sub()}
        onBack={back}
        trailing={
          <Show when={page()}>
            <ThreadSelectionMenu items={props.threadMenu()} variant="glass" class={menuTriggerVariants()} />
          </Show>
        }
        below={
          <Show when={!atTop()}>
            <ThreadNavUp onClick={up} />
          </Show>
        }
      />
      <div data-thread-scroll="" data-post-column="" class={mobile() ? listMobileVariants() : listVariants()}>
        <For each={posts()}>
          {(uid, index) => (
            <>
              <Show when={index() === unread.markerIndex()}>
                <div class={markerVariants()} />
              </Show>
              <Post
                uid={uid}
                forumId={props.forumId}
                threadUid={page()!.uid}
                threadClosed={page()!.closed}
                threadBroom={page()!.opAdmin}
                slug={props.slug}
                threadNum={props.threadNum}
                onEdit={props.onEdit}
              />
            </>
          )}
        </For>
      </div>
      <PostComposer
        mode="reply"
        slug={props.slug}
        threadNum={props.threadNum}
        editingUid={props.editingUid}
        onEditDone={props.onEditDone}
        nav={
          <Show when={!atEnd()}>
            <ThreadNavDown unread={unread.unreadCount()} onClick={down} />
          </Show>
        }
      />
    </section>
  )
}
