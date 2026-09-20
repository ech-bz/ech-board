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
import { canPostAction } from '../../core/intent/capabilities'

const pageVariants = cva('flex min-h-screen min-w-0 flex-col')
const screenVariants = cva('flex min-h-dvh flex-col bg-bg')
const menuTriggerVariants = cva('ml-auto px-1 py-2 hover:bg-hover')
const listVariants = cva('pl-4')
const listMobileVariants = cva('pl-[calc(1rem+env(safe-area-inset-left,0px))] pr-[calc(1rem+env(safe-area-inset-right,0px))] pb-0 pt-2.5')
const markerVariants = cva('my-3 w-full border-t-2 border-dashed border-accent')

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
    items.push({ label: t('mod.moderators'), onClick: () => ea.openThreadModerators() })
    items.push({ label: t('mod.bans'), onClick: () => ea.openBans(t('mod.bans'), current.uid) })
    if (ea.canBanAny()) items.push({ label: t('mod.banUid'), onClick: () => ea.openBanUid() })
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
  const unread = createThreadUnread(() => posts().length)
  let atBottom = false
  let prevLength = 0
  const [atTop, setAtTop] = createSignal(true)
  const [atEnd, setAtEnd] = createSignal(false)
  const measure = () => {
    const max = document.documentElement.scrollHeight - window.innerHeight
    const y = window.scrollY
    atBottom = max <= 0 || y >= max - 0.5
    setAtTop(y <= 1)
    setAtEnd(atBottom)
  }
  window.addEventListener('scroll', measure, { passive: true })
  window.addEventListener('resize', measure)
  onCleanup(() => {
    window.removeEventListener('scroll', measure)
    window.removeEventListener('resize', measure)
  })
  const mounted = requestAnimationFrame(measure)
  onCleanup(() => cancelAnimationFrame(mounted))

  const up = () => window.scrollTo(0, 0)
  const down = () => window.scrollTo(0, document.documentElement.scrollHeight)

  createEffect(() => {
    const length = posts().length
    const previous = prevLength
    prevLength = length
    if (previous === 0 || length <= previous) return
    if (document.visibilityState !== 'visible' || !atBottom) return
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        window.scrollTo(0, document.documentElement.scrollHeight)
        atBottom = true
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
    const composer = document.querySelector<HTMLElement>('[data-composer]')
    if (!composer) return
    const observer = new ResizeObserver(() => {
      measure()
      if (document.visibilityState !== 'visible' || !atBottom) return
      const container = document.querySelector<HTMLElement>('[data-thread-scroll]')
      if (!container) return
      const list = container.querySelectorAll<HTMLElement>('article[data-post]')
      const last = list[list.length - 1]
      if (!last) return
      const rect = last.getBoundingClientRect()
      const composerTop = composer.getBoundingClientRect().top
      if (rect.bottom - composerTop > 0 && rect.top < window.innerHeight) {
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
      <div data-thread-scroll="" class={mobile() ? listMobileVariants() : listVariants()}>
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
