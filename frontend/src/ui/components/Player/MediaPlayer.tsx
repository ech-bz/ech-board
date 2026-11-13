import { createEffect, createSignal, on, onCleanup, onMount, Show, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Button } from '../Button'
import { IconButton } from '../IconButton'
import { DropdownMenu, type MenuItem } from '../Menu/DropdownMenu'
import type { GalleryItem } from '../Gallery/context'
import { useIsMobile } from '../Gallery/useIsMobile'
import { MusicIcon } from '../../icons/MusicIcon'
import { PauseIcon } from '../../icons/PauseIcon'
import { PlayIcon } from '../../icons/PlayIcon'
import { VolumeIcon } from '../../icons/VolumeIcon'
import { VolumeOffIcon } from '../../icons/VolumeOffIcon'
import { useI18n } from '../../i18n'
import { useNav } from '../../nav'
import { canUseNativePip, enterNativePip } from './nativePip'
import { PIP_MAX_WIDTH, PIP_MIN_WIDTH, playerPrefs } from './playerPrefs'

const playerVariants = cva('pointer-events-auto flex flex-col rounded-md border border-separator bg-panel1 text-fg [transform-origin:center]', {
  variants: {
    floating: {
      true: 'fixed right-4 bottom-4 z-overlay shadow-[0_1px_3px_rgba(0,0,0,0.3)] max-[899px]:bottom-[calc(var(--mobile-tab)_+_16px)]',
      false: 'relative',
    },
  },
})
const screenVariants = cva('relative flex-none overflow-hidden rounded-t-[9px] bg-black', {
  variants: {
    floating: {
      true: 'cursor-grab [touch-action:none]',
    },
    dragging: {
      true: 'cursor-grabbing',
    },
  },
})
const statusVariants = cva('absolute inset-0 flex flex-col items-center justify-center gap-2 text-sm text-scrim-fg', {
  variants: {
    kind: {
      loading: 'pointer-events-none',
      error: 'bg-black/70 px-3 text-center',
    },
  },
})
const spinnerVariants = cva('animate-spin rounded-full border-[3px] border-track border-t-fg', {
  variants: {
    small: {
      true: 'size-6',
      false: 'size-10',
    },
  },
})
const resizeGripVariants = cva('absolute right-0 bottom-0 z-1 size-7 cursor-nwse-resize opacity-60 hover:opacity-100 [touch-action:none] after:absolute after:right-1.5 after:bottom-1.5 after:size-2.5 after:border-r-2 after:border-b-2 after:border-scrim-fg after:content-[""]')
const mediaVariants = cva('relative z-1 block size-full select-none object-contain')
const mediaBackdropVariants = cva('pointer-events-none absolute inset-0 z-0 size-full scale-[1.15] object-cover blur-[28px] brightness-[0.65]')
const mediaBackdropShadeVariants = cva('pointer-events-none absolute inset-0 z-0 bg-black/25')
const placeholderVariants = cva('pointer-events-none absolute inset-0 flex items-center justify-center text-scrim-fg', {
  variants: {
    dim: {
      true: 'opacity-0',
    },
  },
})
const toolbarVariants = cva('flex flex-none items-center gap-2 rounded-b-[9px] border-t border-separator bg-panel1 p-2')
const ctrlBtnVariants = cva('bg-panel2 hover:bg-hover [&_svg]:size-4')
const timelineVariants = cva('flex min-w-0 flex-1 flex-row items-center gap-2')
const timeVariants = cva('whitespace-nowrap text-sm leading-none text-secondary [font-variant-numeric:tabular-nums]')
const seekBarVariants = cva('relative h-1 min-w-0 flex-1 cursor-pointer rounded-sm bg-toggle-track [touch-action:none] before:absolute before:-inset-x-2.5 before:-inset-y-2.5 before:content-[""]')
const seekBufferVariants = cva('pointer-events-none absolute inset-y-0 left-0 rounded-sm bg-fg/25')
const seekFillVariants = cva('pointer-events-none absolute inset-y-0 left-0 rounded-sm bg-accent')
const seekMarkVariants = cva('pointer-events-none absolute -top-2 size-0 -translate-x-1/2 border-x-4 border-t-[6px] border-x-transparent border-t-fg')
const volWrapVariants = cva('relative flex flex-none items-center', {
  variants: {
    open: {
      true: 'before:absolute before:inset-x-0 before:bottom-full before:h-25 before:content-[""]',
    },
  },
})
const volPopupVariants = cva('absolute bottom-[calc(100%+8px)] left-1/2 flex w-6 -translate-x-1/2 cursor-pointer justify-center rounded-full border border-separator bg-active py-2.5 transition-opacity duration-150 [touch-action:none]', {
  variants: {
    open: {
      true: 'pointer-events-auto opacity-100',
      false: 'pointer-events-none opacity-0',
    },
  },
})
const volTrackVariants = cva('relative h-volbar w-1 rounded-sm bg-toggle-track')
const volFillVariants = cva('absolute inset-x-0 bottom-0 rounded-sm bg-accent')

const GALLERY_RESERVED = 150
const AUDIO_MIN = 218

export function isPlayableMime(mime: string): boolean {
  return mime.startsWith('video/') || mime.startsWith('audio/')
}

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max)
}

function fmtTime(s: number): string {
  if (!Number.isFinite(s) || s <= 0) return '0:00'
  const m = Math.floor(s / 60)
  const sec = Math.floor(s % 60)
  return `${m}:${String(sec).padStart(2, '0')}`
}

function createMedia(item: GalleryItem, active: boolean): HTMLMediaElement {
  const preload = active ? 'auto' : 'metadata'
  if (item.mime.startsWith('video/')) {
    return (<video class={mediaVariants()} src={item.fullUrl} preload={preload} loop playsinline />) as HTMLVideoElement
  }
  return (<audio src={item.fullUrl} preload={preload} loop />) as HTMLAudioElement
}

export interface MediaPlayerProps {
  item: GalleryItem
  index?: number
  active?: boolean
  autoPip?: boolean
  adopt?: HTMLMediaElement
  floating?: boolean
  rootRef?: (el: HTMLDivElement) => void
  onPip?: (item: GalleryItem, media: HTMLMediaElement) => void
  onNativePip?: (media: HTMLMediaElement) => void
  onToGallery?: (item: GalleryItem, media: HTMLMediaElement) => void
  onLeave?: () => void
  onClose?: () => void
}

export function MediaPlayer(props: MediaPlayerProps) {
  const { t } = useI18n()
  const nav = useNav()
  const mobile = useIsMobile()
  const isVideo = props.item.mime.startsWith('video/')
  const media: HTMLMediaElement = props.adopt ?? createMedia(props.item, props.active ?? true)
  const ratio = isVideo && props.item.width > 0 && props.item.height > 0 ? props.item.width / props.item.height : 16 / 9

  const [playing, setPlaying] = createSignal(!media.paused)
  const [time, setTime] = createSignal(media.currentTime)
  const [dur, setDur] = createSignal(Number.isFinite(media.duration) ? media.duration : 0)
  const [scrubT, setScrubT] = createSignal<number | null>(null)
  const [volOpen, setVolOpen] = createSignal(false)
  const [pos, setPos] = createSignal<{ x: number; y: number } | null>(null)
  const [dragging, setDragging] = createSignal(false)
  const [loading, setLoading] = createSignal(false)
  const [failed, setFailed] = createSignal(false)
  const [bufferedEnd, setBufferedEnd] = createSignal(0)
  const [pipW, setPipW] = createSignal(playerPrefs.pipWidth())

  let root: HTMLDivElement | undefined
  let screen: HTMLDivElement | undefined
  let volTrack: HTMLDivElement | undefined
  let handedOff = false
  let autoPipRequested = false
  let seeking = false
  let seekingVol = false
  let seekPaused = false
  let scrubRatio = 0
  let dragOffset = { x: 0, y: 0 }
  let dragStart: { x: number; y: number; moved: boolean } | null = null
  let resizeStart: { x: number; y: number; w: number; h: number } | null = null
  let loadingTimer: number | undefined

  const duration = () => (dur() > 0 ? dur() : (props.item.durationMs ?? 0) / 1000)
  const displayTime = () => scrubT() ?? time()
  const progress = () => (duration() > 0 ? clamp((displayTime() / duration()) * 100, 0, 100) : 0)
  const bufferedPct = () => (duration() > 0 ? clamp((bufferedEnd() / duration()) * 100, 0, 100) : 0)
  const isMuted = () => playerPrefs.muted() || playerPrefs.volume() === 0

  const toggle = () => {
    if (media.paused) media.play().catch(() => undefined)
    else media.pause()
  }

  const scrubTo = (clientX: number, bar: HTMLElement) => {
    const total = duration()
    if (total <= 0) return
    const rect = bar.getBoundingClientRect()
    scrubRatio = clamp((clientX - rect.left) / rect.width, 0, 1)
    setScrubT(scrubRatio * total)
  }

  const releaseSeek = () => {
    if (!seeking) return
    seeking = false
    const total = duration()
    if (total > 0) media.currentTime = clamp(scrubRatio * total, 0, total)
    setScrubT(null)
    if (seekPaused) {
      seekPaused = false
      media.play().catch(() => undefined)
    }
  }

  const setVolFromY = (clientY: number) => {
    if (!volTrack) return
    const rect = volTrack.getBoundingClientRect()
    const v = clamp(1 - (clientY - rect.top) / rect.height, 0, 1)
    playerPrefs.setVolume(v)
    playerPrefs.setMuted(v === 0)
  }

  const place = (x: number, y: number) => {
    if (!root) return
    const rect = root.getBoundingClientRect()
    setPos({ x: clamp(x, 0, Math.max(0, window.innerWidth - rect.width)), y: clamp(y, 0, Math.max(0, window.innerHeight - rect.height)) })
  }

  const startCustomPip = () => {
    handedOff = true
    props.onPip?.(props.item, media)
  }

  const startPip = () => {
    if (!canUseNativePip(media)) {
      startCustomPip()
      return
    }
    void enterNativePip(media as HTMLVideoElement).then((ok) => {
      if (ok) props.onNativePip?.(media)
      else startCustomPip()
    })
  }

  const retry = () => {
    const resumeAt = media.currentTime
    setFailed(false)
    media.addEventListener(
      'loadedmetadata',
      () => {
        if (resumeAt > 0) media.currentTime = resumeAt
      },
      { once: true },
    )
    media.load()
    media.play().catch(() => undefined)
  }

  const beginDrag = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    if (!props.floating || !root) return
    if (e.pointerType === 'mouse' && e.button !== 0) return
    e.preventDefault()
    const rect = root.getBoundingClientRect()
    dragOffset = { x: e.clientX - rect.left, y: e.clientY - rect.top }
    dragStart = { x: e.clientX, y: e.clientY, moved: false }
    try {
      e.currentTarget.setPointerCapture(e.pointerId)
    } catch {
      return
    }
  }

  const moveDrag = (e: PointerEvent) => {
    if (!dragStart || !root) return
    if (!dragStart.moved) {
      if (Math.hypot(e.clientX - dragStart.x, e.clientY - dragStart.y) < 4) return
      dragStart.moved = true
      const rect = root.getBoundingClientRect()
      setPos({ x: rect.left, y: rect.top })
      setDragging(true)
    }
    place(e.clientX - dragOffset.x, e.clientY - dragOffset.y)
  }

  const endDrag = (e: PointerEvent) => {
    const start = dragStart
    dragStart = null
    setDragging(false)
    if (start && !start.moved && e.type === 'pointerup') toggle()
  }

  const beginResize = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    if (!root) return
    e.preventDefault()
    e.stopPropagation()
    const rect = root.getBoundingClientRect()
    setPos({ x: rect.left, y: rect.top })
    resizeStart = { x: e.clientX, y: e.clientY, w: rect.width, h: rect.height }
    try {
      e.currentTarget.setPointerCapture(e.pointerId)
    } catch {
      return
    }
  }

  const moveResize = (e: PointerEvent) => {
    const start = resizeStart
    const p = pos()
    if (!start || !p || !root) return
    const dx = e.clientX - start.x
    const dy = (e.clientY - start.y) * (start.w / start.h)
    const delta = Math.abs(dx) >= Math.abs(dy) ? dx : dy
    setPipW(clamp(start.w + delta, PIP_MIN_WIDTH, Math.min(PIP_MAX_WIDTH, window.innerWidth - p.x)))
    place(p.x, p.y)
  }

  const endResize = () => {
    if (!resizeStart) return
    resizeStart = null
    playerPrefs.setPipWidth(pipW())
  }

  const reportBug = () => {
    props.onLeave?.()
    nav.navigatePath('/d')
  }

  const toGallery = () => {
    handedOff = true
    props.onToGallery?.(props.item, media)
  }

  const menuItems = (): MenuItem[] => {
    const items: MenuItem[] = []
    if (!props.floating) items.push({ label: t('player.pip'), onClick: startPip })
    if (props.floating && props.onToGallery) items.push({ label: t('player.toGallery'), onClick: toGallery })
    if (props.floating) items.push({ label: t('player.close'), onClick: () => props.onClose?.() })
    items.push({ label: t('player.reportBug'), onClick: reportBug })
    return items
  }

  createEffect(
    on(
      () => props.active ?? true,
      (active) => {
        if (active) media.play().catch(() => undefined)
        else media.pause()
      },
      { defer: !!props.adopt },
    ),
  )

  createEffect(() => {
    const active = props.active ?? true
    if (!active) {
      autoPipRequested = false
      return
    }
    if (!props.autoPip || autoPipRequested || !props.onPip) return
    autoPipRequested = true
    queueMicrotask(() => {
      if ((props.active ?? true) && props.autoPip && !handedOff) startPip()
    })
  })

  createEffect(() => {
    media.muted = playerPrefs.muted()
    media.volume = playerPrefs.volume()
  })

  if (!props.adopt) {
    createEffect(() => {
      media.preload = (props.active ?? true) ? 'auto' : 'metadata'
    })
  }

  createEffect(() => {
    const syncState = () => {
      const hasError = !!media.error
      setFailed(hasError)
      setPlaying(!media.paused)
      const waiting = !hasError && (!media.paused || media.seeking) && media.readyState < 3
      if (waiting) {
        if (loadingTimer === undefined && !loading()) {
          loadingTimer = window.setTimeout(() => {
            loadingTimer = undefined
            setLoading(true)
          }, 250)
        }
      } else {
        if (loadingTimer !== undefined) {
          window.clearTimeout(loadingTimer)
          loadingTimer = undefined
        }
        setLoading(false)
      }
      let end = 0
      const ranges = media.buffered
      for (let i = 0; i < ranges.length; i++) {
        if (ranges.start(i) <= media.currentTime + 0.5 && ranges.end(i) > end) end = ranges.end(i)
      }
      setBufferedEnd(end)
    }
    const onTime = () => setTime(media.currentTime)
    const onMeta = () => {
      setDur(Number.isFinite(media.duration) ? media.duration : 0)
      syncState()
    }
    const stateEvents = ['play', 'pause', 'loadstart', 'loadeddata', 'canplay', 'playing', 'waiting', 'stalled', 'seeking', 'seeked', 'progress', 'emptied', 'error']
    for (const name of stateEvents) media.addEventListener(name, syncState)
    media.addEventListener('timeupdate', onTime)
    media.addEventListener('loadedmetadata', onMeta)
    media.addEventListener('durationchange', onMeta)
    syncState()
    onCleanup(() => {
      for (const name of stateEvents) media.removeEventListener(name, syncState)
      media.removeEventListener('timeupdate', onTime)
      media.removeEventListener('loadedmetadata', onMeta)
      media.removeEventListener('durationchange', onMeta)
      if (loadingTimer !== undefined) {
        window.clearTimeout(loadingTimer)
        loadingTimer = undefined
      }
    })
  })

  createEffect(() => {
    if (!props.floating || !pos()) return
    const onResize = () => {
      const p = pos()
      if (p) place(p.x, p.y)
    }
    window.addEventListener('resize', onResize)
    onCleanup(() => window.removeEventListener('resize', onResize))
  })

  onMount(() => {
    if (!props.adopt) return
    screen?.insertBefore(media, screen.firstChild)
    const wasPlaying = playing()
    queueMicrotask(() => {
      if (wasPlaying && media.paused) media.play().catch(() => undefined)
    })
  })

  onCleanup(() => {
    if (!handedOff) media.pause()
  })

  const rootStyle = (): JSX.CSSProperties => {
    if (props.floating) {
      const p = pos()
      const width = `min(${pipW()}px, calc(100vw - 16px))`
      return p ? { width, left: `${p.x}px`, top: `${p.y}px`, right: 'auto', bottom: 'auto' } : { width }
    }
    return {
      width: `min(100%, calc((100dvh - ${GALLERY_RESERVED}px - env(safe-area-inset-bottom, 0px)) * ${ratio.toFixed(4)}), max(${isVideo && props.item.width > 0 ? props.item.width : 480}px, 480px))`,
      'min-width': 'min(300px, 100%)',
    }
  }

  const screenStyle = (): JSX.CSSProperties => {
    if (!isVideo) return { 'aspect-ratio': `${ratio}`, 'min-width': `${AUDIO_MIN}px`, 'min-height': `${AUDIO_MIN}px` }
    if (props.floating) return { 'aspect-ratio': `${ratio}`, 'max-height': '40dvh' }
    return { 'aspect-ratio': `${ratio}` }
  }

  const stopGesture = (e: Event) => e.stopPropagation()

  return (
    <div
      ref={(el) => {
        root = el
        props.rootRef?.(el)
      }}
      data-index={props.index}
      data-player=""
      class={playerVariants({ floating: !!props.floating })}
      style={rootStyle()}
    >
      <div
        ref={screen}
        class={screenVariants({ floating: props.floating, dragging: dragging() })}
        style={screenStyle()}
        onPointerDown={beginDrag}
        onPointerMove={moveDrag}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
      >
        <Show when={isVideo && props.item.thumbUrl}>
          <img class={mediaBackdropVariants()} src={props.item.thumbUrl} alt="" aria-hidden="true" draggable={false} />
          <div class={mediaBackdropShadeVariants()} />
        </Show>
        {props.adopt ? undefined : media}
        <Show when={!isVideo}>
          <div class={placeholderVariants({ dim: loading() && !failed() })}>
            <MusicIcon width={props.floating ? 40 : 64} height={props.floating ? 40 : 64} />
          </div>
        </Show>
        <Show when={loading() && !failed()}>
          <div class={statusVariants({ kind: 'loading' })} role="status" aria-label={t('player.loading')}>
            <span class={spinnerVariants({ small: !!props.floating })} />
          </div>
        </Show>
        <Show when={failed()}>
          <div
            class={statusVariants({ kind: 'error' })}
            role="alert"
            onPointerDown={stopGesture}
            onMouseDown={stopGesture}
            onTouchStart={stopGesture}
            onTouchMove={stopGesture}
            onTouchEnd={stopGesture}
          >
            <span>{t('player.loadError')}</span>
            <Button size="xs" onClick={retry}>
              {t('player.retry')}
            </Button>
          </div>
        </Show>
        <Show when={props.floating}>
          <div
            class={resizeGripVariants()}
            aria-label={t('player.resize')}
            onPointerDown={beginResize}
            onPointerMove={moveResize}
            onPointerUp={endResize}
            onPointerCancel={endResize}
          />
        </Show>
      </div>
      <div class={toolbarVariants()} onMouseDown={stopGesture} onTouchStart={stopGesture} onTouchMove={stopGesture} onTouchEnd={stopGesture}>
        <IconButton size="md" class={ctrlBtnVariants()} aria-label={playing() ? t('player.pause') : t('player.play')} onClick={toggle}>
          <Show when={playing()} fallback={<PlayIcon />}>
            <PauseIcon />
          </Show>
        </IconButton>
        <div class={timelineVariants()}>
          <div
            class={seekBarVariants()}
            role="slider"
            aria-label={t('player.seek')}
            onPointerDown={(e) => {
              if (!media.paused) {
                seekPaused = true
                media.pause()
              }
              seeking = true
              scrubTo(e.clientX, e.currentTarget)
              try {
                e.currentTarget.setPointerCapture(e.pointerId)
              } catch {
                seeking = true
              }
            }}
            onPointerMove={(e) => {
              if (seeking) scrubTo(e.clientX, e.currentTarget)
            }}
            onPointerUp={releaseSeek}
            onPointerCancel={releaseSeek}
          >
            <div class={seekBufferVariants()} style={{ width: `${bufferedPct()}%` }} />
            <div class={seekFillVariants()} style={{ width: `${progress()}%` }} />
            <span class={seekMarkVariants()} style={{ left: `${progress()}%` }} />
          </div>
          <span class={timeVariants()}>
            {fmtTime(displayTime())} / {fmtTime(duration())}
          </span>
        </div>
        <div
          class={volWrapVariants({ open: volOpen() })}
          onMouseEnter={() => {
            if (!mobile()) setVolOpen(true)
          }}
          onMouseLeave={() => {
            if (!mobile()) setVolOpen(false)
          }}
        >
          <div
            class={volPopupVariants({ open: volOpen() })}
            onPointerDown={(e) => {
              seekingVol = true
              setVolFromY(e.clientY)
              try {
                e.currentTarget.setPointerCapture(e.pointerId)
              } catch {
                seekingVol = true
              }
            }}
            onPointerMove={(e) => {
              if (seekingVol) setVolFromY(e.clientY)
            }}
            onPointerUp={() => {
              seekingVol = false
            }}
            onPointerCancel={() => {
              seekingVol = false
            }}
          >
            <div ref={volTrack} class={volTrackVariants()}>
              <div class={volFillVariants()} style={{ height: `${isMuted() ? 0 : playerPrefs.volume() * 100}%` }} />
            </div>
          </div>
          <IconButton
            size="md"
            class={ctrlBtnVariants()}
            aria-label={isMuted() ? t('player.unmute') : t('player.mute')}
            onClick={() => {
              if (mobile() || playerPrefs.volume() === 0) playerPrefs.setVolume(1)
              playerPrefs.setMuted(!playerPrefs.muted())
            }}
          >
            <Show when={isMuted()} fallback={<VolumeIcon />}>
              <VolumeOffIcon />
            </Show>
          </IconButton>
        </div>
        <DropdownMenu items={menuItems()} triggerSize="md" class={ctrlBtnVariants()} ariaLabel={t('player.options')} />
      </div>
    </div>
  )
}
