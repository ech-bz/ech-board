import { createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show, useContext } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ModWindow } from './ModWindow'
import { matchesQuery, useModFormat } from './format'
import { DropdownMenu, type MenuItem } from '../Menu/DropdownMenu'
import { BanModal } from '../BanModal'
import { ModCheck } from './ModCheck'
import { ModMediaThumb } from './ModMedia'
import { ApiCtx } from '../Gallery/context'
import { useI18n } from '../../i18n'
import { useNav } from '../../nav'
import { threadPath } from '../../route'
import { ROLE_STYLE, type RoleKind } from '../../../core/intent/roles'
import { canBanAny } from '../../../core/intent/capabilities'
import { modData } from '../../mod/modData'
import type { ModMedia, ModPost } from '../../mod/types'
import type { GalleryItem, SourceRect } from '../Gallery/context'

const PAGE = 20

const itemVariants = cva('border-b border-panel3/60 px-4 py-3 [content-visibility:auto] [contain-intrinsic-size:auto_96px] last:border-b-0', {
  variants: {
    selected: { true: 'bg-active' },
    deleted: { true: 'opacity-55' },
  },
})
const headVariants = cva('mb-2 flex items-center gap-2 text-md text-secondary min-[900px]:text-lg')
const metaVariants = cva('flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-0.5')
const numVariants = cva('post-num cursor-pointer whitespace-nowrap font-semibold text-accent no-underline')
const boardVariants = cva('whitespace-nowrap font-semibold text-accent')
const deletedVariants = cva('text-md font-semibold uppercase tracking-wide text-danger')
const bodyVariants = cva('flex flex-col gap-2')
const thumbsVariants = cva('flex flex-wrap gap-2')
const textVariants = cva('min-w-0 whitespace-pre-wrap break-words text-lg', {
  variants: { clamp: { true: 'post-text collapsed' } },
})
const quoteVariants = cva('text-quote')
const expandVariants = cva(
  'absolute bottom-2 left-1/2 inline-flex -translate-x-1/2 cursor-pointer items-center rounded-full border-0 bg-panel3 px-5 py-1.5 text-md text-fg shadow-md hover:bg-hover',
)
const emptyVariants = cva('p-8 text-center text-secondary')

function toGalleryItem(post: ModPost, m: ModMedia, openPost: () => void): GalleryItem {
  return {
    thumbUrl: m.thumbUrl,
    fullUrl: m.fullUrl,
    width: m.width,
    height: m.height,
    mime: m.mime.startsWith('video/') ? 'image/jpeg' : m.mime,
    size: m.size,
    durationMs: null,
    postUid: String(post.num),
    threadUid: String(post.threadNum),
    hash: m.id,
    banned: m.hidden,
    info: { posterUid: post.authorUid, postNum: post.num, board: post.board, ts: post.ts, onOpenPost: openPost },
  }
}

function OverboardItem(props: {
  post: ModPost
  forumId: string
  selected: boolean
  canModerate: boolean
  onToggle: () => void
  menu: MenuItem[]
  onHideMedia: (mediaId: string) => void
}) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const nav = useNav()
  const gallery = useContext(ApiCtx)
  const [expanded, setExpanded] = createSignal(false)
  const [long, setLong] = createSignal(false)
  let textEl: HTMLDivElement | undefined
  onMount(() => {
    if (!textEl) return
    const measure = () => {
      if (!expanded() && textEl) setLong(textEl.scrollHeight > textEl.clientHeight + 1)
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(textEl)
    onCleanup(() => ro.disconnect())
  })
  const goToPost = () => nav.navigate({ page: 'thread', forumId: props.forumId, slug: props.post.board, threadNum: props.post.threadNum, postNum: props.post.num })
  const open = (e: MouseEvent) => {
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
    e.preventDefault()
    goToPost()
  }
  const openMedia = (m: ModMedia, rect: SourceRect) => {
    const live = props.post.media.filter((item) => !item.hidden)
    gallery.open(live.map((item) => toGalleryItem(props.post, item, goToPost)), live.indexOf(m), rect)
  }
  return (
    <article class={itemVariants({ selected: props.selected, deleted: props.post.deleted })}>
      <div class={headVariants()}>
        <Show when={props.canModerate}>
          <ModCheck checked={props.selected} onToggle={props.onToggle} />
        </Show>
        <div class={metaVariants()}>
          <span class={boardVariants()}>/{props.post.board}/</span>
          <Show when={props.post.role}>
            {(role) => (
              <span style={{ color: ROLE_STYLE[role()].color, 'font-weight': ROLE_STYLE[role()].bold ? 700 : 400 }}>{ROLE_STYLE[role()].text}</span>
            )}
          </Show>
          <span class="whitespace-nowrap font-mono text-md">{props.post.authorUid}</span>
          <span class="whitespace-nowrap">
            {t('post.anon')} {fmt.full(props.post.ts)}
          </span>
          <a class={numVariants()} href={threadPath(props.forumId, props.post.board, props.post.threadNum, props.post.num)} onClick={open}>
            #{props.post.num}
          </a>
          <Show when={props.post.deleted}>
            <span class={deletedVariants()}>{t('post.deleted')}</span>
          </Show>
        </div>
        <Show when={props.canModerate}>
          <DropdownMenu items={props.menu} triggerSize="xs" iconSize={14} triggerShape="rounded" class="ml-auto self-start" />
        </Show>
      </div>
      <div class={bodyVariants()}>
        <Show when={props.post.media.length > 0}>
          <div class={thumbsVariants()}>
            <For each={props.post.media}>
              {(m) => (
                <ModMediaThumb
                  thumbUrl={m.thumbUrl}
                  fullUrl={m.fullUrl}
                  mime={m.mime}
                  width={m.width}
                  height={m.height}
                  size={m.size}
                  hidden={m.hidden}
                  onDelete={props.canModerate && !props.post.deleted ? () => props.onHideMedia(m.id) : undefined}
                  deleteLabel={t('modUi.hideMedia')}
                  onOpen={(rect) => openMedia(m, rect)}
                />
              )}
            </For>
          </div>
        </Show>
        <div class="relative min-w-0">
          <Show when={props.post.replyTo}>
            <div class={quoteVariants()}>&gt;&gt;{props.post.replyTo}</div>
          </Show>
          <div ref={textEl} class={textVariants({ clamp: !expanded() })}>
            {props.post.text}
          </div>
          <Show when={long() && !expanded()}>
            <button type="button" class={expandVariants()} onClick={() => setExpanded(true)}>
              {t('modUi.showFull')}
            </button>
          </Show>
        </div>
      </div>
    </article>
  )
}

export function OverboardPanel(props: { forumId: string; roleKinds: RoleKind[]; canModerate: boolean }) {
  const { t } = useI18n()
  const nav = useNav()
  const [query, setQuery] = createSignal('')
  const [limit, setLimit] = createSignal(PAGE)
  const [selected, setSelected] = createSignal<Set<number>>(new Set())
  const [ban, setBan] = createSignal<{ posts: ModPost[]; deleteToo: boolean } | null>(null)
  let scroller: HTMLDivElement | undefined
  let sentinel: HTMLDivElement | undefined

  const filtered = createMemo(() =>
    modData.posts().filter((p) => matchesQuery(query(), p.text, p.authorUid, p.num, p.threadNum, p.board, p.media.map((m) => m.name).join(' '))),
  )
  const visible = createMemo(() => filtered().slice(0, limit()))
  const byNum = (num: number) => modData.posts().find((p) => p.num === num)
  const selectedPosts = () => [...selected()].map(byNum).filter((p): p is ModPost => !!p)

  createEffect(
    on(
      query,
      () => {
        setLimit(PAGE)
        scroller?.scrollTo({ top: 0 })
      },
      { defer: true },
    ),
  )
  createEffect(() => {
    if (limit() >= filtered().length || !sentinel) return
    const observer = new IntersectionObserver(([entry]) => entry.isIntersecting && setLimit((n) => n + PAGE), { root: scroller, rootMargin: '400px' })
    observer.observe(sentinel)
    onCleanup(() => observer.disconnect())
  })

  const toggle = (num: number) =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(num)) next.delete(num)
      else next.add(num)
      return next
    })
  const clear = () => setSelected(new Set<number>())
  const canBan = () => props.canModerate && canBanAny(props.roleKinds)

  const menuOf = (post: ModPost): MenuItem[] => {
    const items: MenuItem[] = [
      {
        label: t('modUi.openInThread'),
        onClick: () => nav.navigate({ page: 'thread', forumId: props.forumId, slug: post.board, threadNum: post.threadNum, postNum: post.num }),
      },
    ]
    if (post.deleted) items.push({ label: t('mod.restore'), onClick: () => modData.restorePosts([post.num]) })
    else {
      items.push({ label: t('mod.delete'), danger: true, onClick: () => modData.deletePosts([post.num]) })
      const live = post.media.filter((m) => !m.hidden)
      live.forEach((m, i) =>
        items.push({ label: live.length > 1 ? t('modUi.hideMediaN', { n: i + 1 }) : t('modUi.hideMedia'), onClick: () => modData.hideMedia(post.num, m.id) }),
      )
      if (canBan()) {
        items.push({ label: t('mod.ban'), danger: true, onClick: () => setBan({ posts: [post], deleteToo: false }) })
        items.push({ label: t('modUi.banAndDelete'), danger: true, onClick: () => setBan({ posts: [post], deleteToo: true }) })
      }
    }
    items.push({
      label: t('mod.selectAllFromAuthor'),
      onClick: () => setSelected((prev) => new Set([...prev, ...modData.posts().filter((p) => p.authorUid === post.authorUid).map((p) => p.num)])),
    })
    return items
  }

  const bulkItems = (): MenuItem[] => {
    const posts = selectedPosts()
    const dead = posts.filter((p) => p.deleted)
    const live = posts.filter((p) => !p.deleted)
    const items: MenuItem[] = []
    if (dead.length) items.push({ label: t('select.restoreSelected', { n: dead.length }), onClick: () => (modData.restorePosts(dead.map((p) => p.num)), clear()) })
    if (live.length) items.push({ label: t('select.deleteSelected', { n: live.length }), danger: true, onClick: () => (modData.deletePosts(live.map((p) => p.num)), clear()) })
    if (canBan() && live.length) {
      items.push({ label: t('select.banSelected', { n: live.length }), danger: true, onClick: () => setBan({ posts: live, deleteToo: false }) })
      items.push({ label: t('select.deleteBanSelected', { n: live.length }), danger: true, onClick: () => setBan({ posts: live, deleteToo: true }) })
    }
    items.push({ label: t('select.clear'), onClick: clear })
    return items
  }

  return (
    <>
      <ModWindow
        framed
        search={query()}
        onSearch={setQuery}
        scrollRef={(el) => (scroller = el)}
        trailing={
          <Show when={selected().size > 0}>
            <DropdownMenu
              items={bulkItems()}
              ariaLabel={t('modUi.selected', { n: selected().size })}
              icon={<span class="text-md font-semibold">{selected().size}</span>}
              triggerSize="md"
              triggerVariant="glass"
            />
          </Show>
        }
      >
        <Show when={filtered().length > 0} fallback={<div class={emptyVariants()}>{t('modUi.empty')}</div>}>
          <For each={visible()}>
            {(post) => (
              <OverboardItem
                post={post}
                forumId={props.forumId}
                selected={selected().has(post.num)}
                canModerate={props.canModerate}
                onToggle={() => toggle(post.num)}
                menu={menuOf(post)}
                onHideMedia={(mediaId) => modData.hideMedia(post.num, mediaId)}
              />
            )}
          </For>
        </Show>
        <div ref={sentinel} class="h-px" />
      </ModWindow>
      <Show when={ban()}>
        {(target) => (
          <BanModal
            mode="post"
            roleKinds={props.roleKinds}
            onClose={() => setBan(null)}
            onBan={(scope, mask, _durationMs, reason) => {
              modData.banPosts(target().posts, scope, mask, reason)
              if (target().deleteToo) modData.deletePosts(target().posts.map((p) => p.num))
              clear()
              setBan(null)
            }}
          />
        )}
      </Show>
    </>
  )
}
