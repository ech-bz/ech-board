import { createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { A } from '@solidjs/router'
import { ROLE_STYLE } from '../../core/intent/roles'
import { fromHex } from '../../core/intent/crypto'
import { sendReactionIntent } from '../../core/intent/reaction'
import { parseMoveAbort } from '../../core/intent/errors'
import { postLineCount } from '../../core/posts'
import { PostContent } from './PostContent'
import { PostMenu } from './PostMenu'
import { ReactionPicker } from './ReactionPicker'
import { MediaBox } from './MediaBox'
import { LockIcon } from '../icons/LockIcon'
import { PinIcon } from '../icons/PinIcon'
import { BroomIcon } from '../icons/BroomIcon'
import { useGallery } from './Gallery/useGallery'
import { useIsMobile } from './Gallery/useIsMobile'
import { PostRefLink } from '../postTooltip/PostRefLink'
import { reportError } from '../errors'
import { threadPath } from '../route'
import { useNav } from '../nav'
import { useI18n } from '../i18n'
import { useRecord, useStaticInfo, useStore } from '../store'
import { usePostSelection } from '../postSelection/PostSelectionProvider'
import { useComposerBridge } from '../composerBridge'
import type { PostRecord } from '../../store/records'
import type { DecryptContext } from '../../core/intent/decrypt'

export interface PostViewCtx {
  decryptCtx: DecryptContext
  relayUrl: string
  forumId: string
  boardUid: string
}

const postVariants = cva('relative px-3 py-2.5 w-fit max-w-post rounded-lg text-fg border-0 mb-2 will-change-transform', {
  variants: {
    variant: {
      default: 'bg-panel1 [--post-bg:var(--color-panel1)]',
      op: 'bg-panel1 w-auto [--post-bg:var(--color-panel1)]',
      mine: 'bg-mine [--post-bg:var(--color-mine)]',
      repliedMine: 'bg-replied-mine [--post-bg:var(--color-replied-mine)]',
    },
    selected: {
      true: 'outline outline-accent -outline-offset-[1.5px] outline-[1.5px]',
    },
    tooltip: {
      true: 'outline outline-glass2 -outline-offset-[1px] outline-[1px]',
    },
  },
  defaultVariants: { variant: 'default' },
})

const postTextVariants = cva('post-text whitespace-pre-wrap break-words', {
  variants: {
    state: {
      collapsed: 'collapsed',
      expanded: 'expanded',
    },
  },
  defaultVariants: { state: 'expanded' },
})

const headerVariants = cva('mb-1.5 flex items-center gap-1.5 text-sm text-secondary')
const headerMetaVariants = cva('flex min-w-0 flex-1 flex-wrap items-center gap-1.5')
const headerItemVariants = cva('whitespace-nowrap')
const tripcodeVariants = cva('whitespace-nowrap text-success')
const flagVariants = cva('h-3.5 w-auto flex-none rounded-xs')
const checkboxWrapVariants = cva('relative flex flex-none cursor-pointer items-center')
const checkboxVariants = cva('relative size-4 cursor-pointer appearance-none rounded-sm border border-separator bg-panel2 checked:border-accent checked:bg-accent')
const numVariants = cva('post-num bg-transparent p-0 cursor-pointer whitespace-nowrap text-accent [font:inherit]')
const numTextVariants = cva('whitespace-nowrap text-accent')
const relNumVariants = cva('text-quote')
const deletedVariants = cva('text-xs font-semibold uppercase tracking-wide text-danger')
const bannedNoteVariants = cva('mt-5 text-warn italic')
const lockIconVariants = cva('inline-flex size-4 flex-none align-middle text-danger')
const pinIconVariants = cva('inline-flex size-4 flex-none align-middle text-success')
const broomIconVariants = cva('inline-flex size-4 flex-none align-middle text-[#ff6600]')
const MEDIA_FADE = 48
const MEDIA_THUMB_MIN = 24
const mediaWrapVariants = cva('mb-2')
const mediaRowVariants = cva('flex select-none items-center gap-1 overflow-x-auto [overscroll-behavior-x:contain] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden', {
  variants: {
    dragging: {
      true: '[&_*]:cursor-grabbing',
    },
  },
})
const mediaTrackVariants = cva('mt-1.5 h-1 w-full rounded-sm bg-chip')
const mediaThumbVariants = cva('relative h-1 rounded-sm bg-toggle-track transition-colors duration-150 hover:bg-track before:absolute before:-inset-y-1 before:inset-x-0 before:content-[""]')
const expandRowVariants = cva('mt-2 text-center')
const expandBtnVariants = cva('inline-flex cursor-pointer items-center justify-center rounded-full border-0 bg-chip px-2.5 py-1.5 text-sm text-fg')
const reactionsRowVariants = cva('mt-2 flex flex-wrap items-center gap-1.5')
const reactionVariants = cva('inline-flex cursor-pointer items-center justify-center rounded-full border-0 px-2 py-1 gap-1 text-sm text-fg text-bold leading-none hover:bg-hover', {
  variants: {
    active: {
      true: 'bg-reaction',
      false: 'bg-chip',
    },
  },
  defaultVariants: { active: false },
})
const reactionImgVariants = cva('size-4')
const refsRowVariants = cva('mt-2 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-secondary')
const refVariants = cva('cursor-pointer text-accent')

export function PostBody(props: {
  record: PostRecord
  ctx: PostViewCtx
  threadClosed: boolean
  threadBroom: boolean
  pinned?: boolean
  slug?: string
  threadNum?: number
  boardPage?: boolean
  tooltip?: boolean
  onEdit?: (uid: string) => void
  menu?: boolean
}) {
  const store = useStore()
  const staticInfo = useStaticInfo()
  const nav = useNav()
  const { t, months, expandFolds } = useI18n()
  const { openMedia } = useGallery()
  const selection = usePostSelection()
  const composer = useComposerBridge()
  const mobile = useIsMobile()
  const [reactBusy, setReactBusy] = createSignal(false)
  const [expanded, setExpanded] = createSignal(false)
  let postEl: HTMLElement | undefined
  let swipe: { x: number; y: number; mode: 'pending' | 'swipe' | 'vertical'; el: HTMLElement | null } | null = null
  let swipeAnim: Animation | null = null
  const selectable = () => !!selection?.canSelect()
  const selected = () => !!selection?.selected().has(props.record.uid)
  const sameBubbles = (a: PostRecord['reactions'], b: PostRecord['reactions']): boolean =>
    a.length === b.length && a.every((item, i) => item.hex === b[i].hex && item.count === b[i].count)
  let shownBubbles = props.record.reactions
  let shownRefs = props.record.refs.length
  let shownMedia = props.record.media.length
  let shownText = props.record.plain.length
  let growPending = false
  let growScrollY = 0
  createEffect(() => {
    const record = props.record
    if (
      !sameBubbles(shownBubbles, record.reactions) ||
      shownRefs !== record.refs.length ||
      shownMedia !== record.media.length ||
      shownText !== record.plain.length
    ) {
      growPending = true
      growScrollY = window.scrollY
    }
    shownBubbles = record.reactions
    shownRefs = record.refs.length
    shownMedia = record.media.length
    shownText = record.plain.length
  })
  createEffect(() => {
    const element = postEl
    if (!element || props.tooltip) return
    let primed = false
    let height = 0
    const observer = new ResizeObserver(() => {
      const rect = element.getBoundingClientRect()
      if (!primed) {
        primed = true
        height = rect.height
        return
      }
      const delta = rect.height - height
      height = rect.height
      if (delta === 0 || !growPending) return
      growPending = false
      if (window.scrollY !== growScrollY) return
      if (delta < 0 && window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 0.5) return
      const grownTop = rect.bottom - delta
      if (grownTop >= window.innerHeight || rect.bottom <= 0) return
      window.scrollBy(0, delta)
    })
    observer.observe(element)
    onCleanup(() => observer.disconnect())
  })
  const [mediaEl, setMediaEl] = createSignal<HTMLDivElement | null>(null)
  const coarsePointer = window.matchMedia('(pointer: coarse)')
  const [mediaBox, setMediaBox] = createSignal({ left: 0, client: 0, scroll: 0 })
  const [mediaDragging, setMediaDragging] = createSignal(false)
  let mediaDrag: { x: number; left: number; moved: boolean } | null = null
  let mediaThumbDrag: { x: number; left: number } | null = null
  let mediaSuppressClick = false
  const mediaMax = () => Math.max(0, mediaBox().scroll - mediaBox().client)
  const syncMediaBox = () => {
    const el = mediaEl()
    if (!el) return
    const scroll = el.scrollWidth
    const max = Math.max(0, scroll - el.clientWidth)
    setMediaBox({ left: Math.min(Math.max(el.scrollLeft, 0), max), client: el.clientWidth, scroll })
  }
  const mediaMask = () => {
    const box = mediaBox()
    if (box.scroll <= box.client) return undefined
    const start = box.left > 1
    const end = box.left < box.scroll - box.client - 1
    if (!start && !end) return undefined
    const left = `transparent 0, #000 ${MEDIA_FADE}px`
    const right = `#000 calc(100% - ${MEDIA_FADE}px), transparent 100%`
    if (start && end) return `linear-gradient(to right, ${left}, ${right})`
    if (end) return `linear-gradient(to right, #000 0, ${right})`
    return `linear-gradient(to right, ${left}, #000 100%)`
  }
  const mediaThumbGeom = () => {
    const box = mediaBox()
    const max = mediaMax()
    if (max <= 0) return null
    const width = Math.max(MEDIA_THUMB_MIN, (box.client / box.scroll) * box.client)
    return { width, x: (box.left / max) * (box.client - width) }
  }
  const onMediaPointerDown = (e: PointerEvent) => {
    const el = mediaEl()
    if (coarsePointer.matches || !el || e.button !== 0 || mediaMax() <= 0) return
    mediaSuppressClick = false
    mediaDrag = { x: e.clientX, left: el.scrollLeft, moved: false }
  }
  const onMediaPointerMove = (e: PointerEvent) => {
    const el = mediaEl()
    const drag = mediaDrag
    if (!el || !drag) return
    const dx = e.clientX - drag.x
    if (!drag.moved) {
      if (Math.abs(dx) < 4) return
      drag.moved = true
      setMediaDragging(true)
      el.setPointerCapture(e.pointerId)
    }
    el.scrollLeft = drag.left - dx
  }
  const onMediaPointerUp = (e: PointerEvent) => {
    const el = mediaEl()
    const drag = mediaDrag
    mediaDrag = null
    if (el && el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId)
    if (drag?.moved) mediaSuppressClick = true
    setMediaDragging(false)
  }
  const onMediaClickCapture = (e: MouseEvent) => {
    if (!mediaSuppressClick) return
    mediaSuppressClick = false
    e.preventDefault()
    e.stopPropagation()
  }
  const onThumbPointerDown = (e: PointerEvent) => {
    if (coarsePointer.matches || e.button !== 0 || mediaMax() <= 0) return
    mediaThumbDrag = { x: e.clientX, left: mediaBox().left }
    e.currentTarget instanceof HTMLElement && e.currentTarget.setPointerCapture(e.pointerId)
    e.stopPropagation()
  }
  const onThumbPointerMove = (e: PointerEvent) => {
    const el = mediaEl()
    const drag = mediaThumbDrag
    const geom = mediaThumbGeom()
    if (!el || !drag || !geom) return
    const span = mediaBox().client - geom.width
    if (span <= 0) return
    el.scrollLeft = drag.left + ((e.clientX - drag.x) / span) * mediaMax()
  }
  const onThumbPointerUp = (e: PointerEvent) => {
    mediaThumbDrag = null
    const target = e.currentTarget
    if (target instanceof HTMLElement && target.hasPointerCapture(e.pointerId)) target.releasePointerCapture(e.pointerId)
  }
  createEffect(() => {
    const el = mediaEl()
    if (!el) return
    const observer = new ResizeObserver(syncMediaBox)
    observer.observe(el)
    el.addEventListener('scroll', syncMediaBox, { passive: true })
    el.addEventListener('click', onMediaClickCapture, true)
    syncMediaBox()
    onCleanup(() => {
      observer.disconnect()
      el.removeEventListener('scroll', syncMediaBox)
      el.removeEventListener('click', onMediaClickCapture, true)
    })
  })
  const insertNo = () => {
    const quote = composer.takeQuote()
    composer.insert(quote ? `>>${props.record.number}\n${quote}` : `>>${props.record.number}`)
  }
  const springBack = (element: HTMLElement | null) => {
    if (!element) return
    if (swipeAnim) swipeAnim.cancel()
    swipeAnim = element.animate([{ transform: element.style.transform }, { transform: 'translateX(0px)' }], { duration: 260, easing: 'ease-out' })
    swipeAnim.onfinish = () => {
      element.style.transform = ''
    }
  }
  const swipeAllowed = () => mobile() && !props.tooltip && !props.boardPage
  const onTouchStart = (e: TouchEvent) => {
    swipe = null
    if (!swipeAllowed()) return
    const target = e.target
    const carousel = target instanceof Element ? target.closest<HTMLElement>('[data-media-row]') : null
    if (carousel && carousel.scrollWidth > carousel.clientWidth) return
    const touch = e.touches[0]
    if (!touch) return
    if (swipeAnim) {
      swipeAnim.cancel()
      swipeAnim = null
    }
    swipe = { x: touch.clientX, y: touch.clientY, mode: 'pending', el: e.currentTarget as HTMLElement }
  }
  createEffect(() => {
    const el = postEl
    if (!el || !swipeAllowed()) return
    const onMove = (e: TouchEvent) => {
      const s = swipe
      if (!s || s.mode === 'vertical') return
      const touch = e.touches[0]
      if (!touch) return
      const dx = touch.clientX - s.x
      const dy = touch.clientY - s.y
      if (s.mode === 'pending') {
        if (Math.abs(dx) < 12 && Math.abs(dy) < 12) return
        s.mode = dx < 0 && -dx > Math.abs(dy) * 1.5 ? 'swipe' : 'vertical'
        if (s.mode === 'vertical') return
      }
      if (!s.el) return
      const raw = dx < -110 ? -110 + (dx + 110) * 0.3 : dx > 110 ? 110 + (dx - 110) * 0.3 : dx
      s.el.style.transform = `translateX(${Math.min(raw, 0)}px)`
    }
    el.addEventListener('touchmove', onMove, { passive: true })
    onCleanup(() => el.removeEventListener('touchmove', onMove))
  })
  const onTouchEnd = (e: TouchEvent) => {
    const s = swipe
    if (!s) return
    swipe = null
    const touch = e.changedTouches[0]
    const dx = touch ? touch.clientX - s.x : 0
    if (s.mode === 'swipe' && -dx > 70 && !props.boardPage) insertNo()
    springBack(s.el)
  }
  const onTouchCancel = () => {
    if (!swipe) return
    springBack(swipe.el)
    swipe = null
  }
  const link = (number: number) =>
    props.slug !== undefined && props.threadNum !== undefined
      ? threadPath(props.ctx.forumId, props.slug, props.threadNum, number)
      : undefined
  const gotoPost = (e: MouseEvent, number: number) => {
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
    if (props.slug === undefined || props.threadNum === undefined) return
    e.preventDefault()
    nav.navigate({ page: 'thread', forumId: props.ctx.forumId, slug: props.slug, threadNum: props.threadNum, postNum: number })
  }
  const lineCount = createMemo(() => postLineCount(props.record.parts ?? [], expandFolds()))
  const long = () => lineCount() > 15
  const fullTime = () => {
    if (!props.record.timestampMs) return ''
    const d = new Date(props.record.timestampMs)
    const pad = (n: number) => n.toString().padStart(2, '0')
    return `${pad(d.getDate())} ${months()[d.getMonth()]} ${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  }
  const react = async (hex: string) => {
    const rec = props.record
    if (reactBusy()) return
    const current = rec.myReaction
    setReactBusy(true)
    store.setPendingReaction(rec.uid, current === hex ? null : hex, rec.myReactionBase)
    try {
      await sendReactionIntent(
        {
          relayUrl: props.ctx.relayUrl,
          forumId: props.ctx.forumId,
          nonceShardsId: store.info()?.nonceShardsId ?? '',
          boardUid: props.ctx.boardUid,
        },
        rec.threadUid,
        rec.post,
        current ? fromHex(current) : null,
        fromHex(hex),
      )
    } catch (e) {
      store.clearPendingReaction(rec.uid)
      reportError(parseMoveAbort(e)?.message ?? String(e))
    } finally {
      setReactBusy(false)
    }
  }
  return (
    <article
      id={`post-${props.record.number}`}
      data-post=""
      ref={postEl}
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
      onTouchCancel={onTouchCancel}
      class={postVariants({
        variant: props.record.op ? 'op' : props.record.mine ? 'mine' : props.record.repliedMine ? 'repliedMine' : 'default',
        selected: selected(),
        tooltip: !!props.tooltip,
      })}
    >
      <div class={headerVariants()}>
        <Show when={selectable()}>
          <span
            role="checkbox"
            aria-checked={selected()}
            class={checkboxWrapVariants()}
            onClick={(e) => {
              e.stopPropagation()
              selection?.toggle(props.record.uid, e.shiftKey)
            }}
          >
            <input type="checkbox" class={checkboxVariants()} checked={selected()} readOnly />
          </span>
        </Show>
        <div class={headerMetaVariants()}>
          <Show when={props.record.role}>
            {(role) => (
              <span class={headerItemVariants()} style={{ color: ROLE_STYLE[role()!].color, 'font-weight': ROLE_STYLE[role()!].bold ? 700 : 400 }}>
                {ROLE_STYLE[role()!].text}
              </span>
            )}
          </Show>
          <span>
            {props.record.name ?? t('post.anon')}
            <Show when={props.record.tripText}>
              {(trip) => (
                <span class={tripcodeVariants()}>
                  {props.record.tripSecured ? '!!' : '!'}
                  {trip()}
                </span>
              )}
            </Show>
          </span>
          <Show when={props.record.flagUrl}>{(url) => <img class={flagVariants()} src={url()} alt="" />}</Show>
          <span class={headerItemVariants()}>{fullTime()}</span>
          <Show when={link(props.record.number)} fallback={<span class={numTextVariants()}>№{props.record.number}</span>}>
            {(href) => (
              <Show
                when={props.boardPage}
                fallback={
                  <span class={numTextVariants()}>
                    <A class={numVariants()} href={href()} noScroll onClick={(e) => gotoPost(e, props.record.number)}>
                      №
                    </A>
                    <button type="button" class={numVariants()} onClick={insertNo}>
                      {props.record.number}
                    </button>
                  </span>
                }
              >
                <A class={numVariants()} href={href()} noScroll onClick={(e) => gotoPost(e, props.record.number)}>
                  №{props.record.number}
                </A>
              </Show>
            )}
          </Show>
          <Show when={!props.record.deleted && props.record.relNum !== null}>
            <span class={relNumVariants()}>{props.record.relNum}</span>
          </Show>
          <Show when={props.record.deleted}>
            <span class={deletedVariants()}>{t('post.deleted')}</span>
          </Show>
          <Show when={props.record.op && props.threadBroom}>
            <BroomIcon width={16} height={16} class={broomIconVariants()} />
          </Show>
          <Show when={props.record.op && props.threadClosed}>
            <LockIcon width={16} height={16} class={lockIconVariants()} />
          </Show>
          <Show when={props.pinned}>
            <PinIcon width={16} height={16} class={pinIconVariants()} />
          </Show>
        </div>
        <Show when={props.menu !== false}>
          <PostMenu
            record={props.record}
            closed={props.threadClosed}
            threadObject={props.record.op ? (store.threadObject(props.record.threadUid) ?? undefined) : undefined}
            onEdit={props.onEdit}
            slug={props.slug}
            threadNum={props.threadNum}
          />
        </Show>
      </div>
      <Show when={props.record.media.length > 0}>
        <div class={mediaWrapVariants()}>
          <div
            ref={setMediaEl}
            data-media-row=""
            class={mediaRowVariants({ dragging: mediaDragging() })}
            style={{ 'mask-image': mediaMask(), '-webkit-mask-image': mediaMask() }}
            onPointerDown={onMediaPointerDown}
            onPointerMove={onMediaPointerMove}
            onPointerUp={onMediaPointerUp}
            onPointerCancel={onMediaPointerUp}
          >
            <For each={props.record.media}>{(item) => <MediaBox m={item} record={props.record} onOpen={openMedia} />}</For>
          </div>
          <Show when={mediaThumbGeom()}>
            {(geom) => (
              <div class={mediaTrackVariants()}>
                <div
                  class={mediaThumbVariants()}
                  style={{ width: `${geom().width}px`, transform: `translateX(${geom().x}px)` }}
                  onPointerDown={onThumbPointerDown}
                  onPointerMove={onThumbPointerMove}
                  onPointerUp={onThumbPointerUp}
                  onPointerCancel={onThumbPointerUp}
                />
              </div>
            )}
          </Show>
        </div>
      </Show>
      <div class={postTextVariants({ state: long() && !expanded() ? 'collapsed' : 'expanded' })}>
        <Show when={props.record.parts} fallback={<span>{props.record.plain}</span>}>
          {(parts) => (
            <PostContent
              parts={parts()}
              senderTweak={props.record.senderTweak}
              senderPub={props.record.senderPk}
              decryptCtx={props.ctx.decryptCtx}
              relayUrl={props.ctx.relayUrl}
              forumId={props.ctx.forumId}
              boardUid={props.ctx.boardUid}
              slug={props.slug}
              threadNum={props.threadNum}
            />
          )}
        </Show>
      </div>
      <Show when={long()}>
        <div class={expandRowVariants()}>
          <button type="button" class={expandBtnVariants()} onClick={() => setExpanded(!expanded())}>
            {expanded() ? t('post.collapse') : t('post.expand', { n: lineCount() })}
          </button>
        </div>
      </Show>
      <Show when={props.record.banned}>
        <div class={bannedNoteVariants()}>{t('post.authorBanned')}</div>
      </Show>
      <Show when={props.record.reactions.length > 0}>
        <div class={reactionsRowVariants()}>
          <For each={props.record.reactions}>
            {(item) => (
              <button
                type="button"
                disabled={reactBusy()}
                onClick={() => void react(item.hex)}
                class={reactionVariants({ active: item.hex === props.record.myReaction })}
              >
                <img src={item.url} alt="" class={reactionImgVariants()} />
                {item.count}
              </button>
            )}
          </For>
        </div>
      </Show>
      <Show when={staticInfo()}>
        {(info) => (
          <ReactionPicker
            glyph={props.record.reactionGlyph}
            relayUrl={props.ctx.relayUrl}
            boardUid={props.ctx.boardUid}
            reactions={info().boardReactions}
            busy={reactBusy()}
            onPick={(hex) => void react(hex)}
          />
        )}
      </Show>
      <Show when={props.record.refs.length > 0}>
        <div class={refsRowVariants()}>
          <For each={props.record.refs}>
            {(ref) => (
              <Show when={link(ref.number)} fallback={<span class={refVariants()}>{`>>${ref.number}`}</span>}>
                {(href) => (
                  <PostRefLink
                    class={refVariants()}
                    href={href()}
                    label={`>>${ref.number}`}
                    target={{ postUid: ref.postUid, number: ref.number }}
                    onClick={(e) => gotoPost(e, ref.number)}
                  />
                )}
              </Show>
            )}
          </For>
        </div>
      </Show>
    </article>
  )
}

export function Post(props: {
  uid: string
  forumId: string
  threadUid: string
  threadClosed: boolean
  threadBroom: boolean
  slug?: string
  threadNum?: number
  onEdit?: (uid: string) => void
}) {
  const store = useStore()
  const record = useRecord(props.uid)
  return (
    <Show when={record()}>
      {(rec) => (
        <Show when={store.decryptCtx() && store.info()}>
          <PostBody
            record={rec()}
            ctx={{
              decryptCtx: store.decryptCtx()!,
              relayUrl: store.info()!.relayUrl,
              forumId: store.info()!.forumId,
              boardUid: store.info()!.boardUid,
            }}
            threadClosed={props.threadClosed}
            threadBroom={props.threadBroom}
            slug={props.slug}
            threadNum={props.threadNum}
            onEdit={props.onEdit}
          />
        </Show>
      )}
    </Show>
  )
}
