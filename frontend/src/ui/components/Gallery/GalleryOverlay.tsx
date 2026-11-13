import { createEffect, createSignal, on, onCleanup, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import type { GalleryItem, SourceRect } from './context'
import { PlayIcon } from '../../icons/PlayIcon'
import { MusicIcon } from '../../icons/MusicIcon'
import { MediaPlayer, isPlayableMime } from '../Player/MediaPlayer'
import { playerPrefs } from '../Player/playerPrefs'

const galleryOverlayVariants = cva('fixed inset-0 z-overlay flex flex-col bg-scrim transition-colors duration-200', {
  variants: {
    hidden: {
      true: 'pointer-events-none opacity-0',
    },
  },
})
const galleryStageVariants = cva('pointer-events-auto min-h-0 flex-1 overflow-hidden [touch-action:none]')
const galleryTrackVariants = cva('pointer-events-none flex h-full w-full gap-4 transition-transform duration-200')
const gallerySlideVariants = cva('pointer-events-none flex h-full flex-[0_0_100%] items-center justify-center')
const galleryMediaVariants = cva('pointer-events-auto max-h-full max-w-full touch-none select-none object-contain [transform-origin:center]')
const galleryBottomVariants = cva('pointer-events-none absolute inset-x-0 bottom-0 flex flex-col items-center gap-3')
const galleryThumbsVariants = cva('flex w-full gap-2 overflow-x-auto bg-shade px-3 pt-2.5 pb-[calc(10px+env(safe-area-inset-bottom,0px))] opacity-0 transition-opacity duration-150', {
  variants: {
    visible: {
      true: 'pointer-events-auto opacity-100',
      false: 'pointer-events-none',
    },
  },
})
const galleryThumbVariants = cva('relative h-14 w-14 flex-none cursor-pointer overflow-hidden rounded-sm border-2 bg-bg opacity-70 hover:opacity-100', {
  variants: {
    active: {
      true: 'border-accent opacity-100',
      false: 'border-transparent',
    },
  },
})
const galleryThumbImgVariants = cva('size-full object-cover')
const galleryThumbPlayVariants = cva('pointer-events-none absolute left-1/2 top-1/2 flex size-6 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-shade text-scrim-fg [&_svg]:ml-0.5 [&_svg]:size-3.5')
const galleryThumbAudioVariants = cva('flex size-full items-center justify-center bg-bg text-fg [&_svg]:size-5')
const galleryThumbPdfVariants = cva('flex size-full items-center justify-center bg-bg text-xs font-bold text-fg')
const galleryPdfCardVariants = cva('pointer-events-auto flex max-h-full max-w-full flex-col items-center justify-center rounded-lg bg-panel1 px-10 py-8 text-2xl font-bold text-accent no-underline')

function findMediaRect(thumbUrl: string): SourceRect | null {
  const nodes = document.querySelectorAll<HTMLImageElement>('[data-media] img')
  for (const img of nodes) {
    if (img.getAttribute('src') === thumbUrl || img.src === thumbUrl) {
      const card = img.closest('[data-media]')
      if (card) {
        const r = card.getBoundingClientRect()
        return { left: r.left, top: r.top, width: r.width, height: r.height }
      }
    }
  }
  return null
}

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max)
}

const TRACK_GAP = 16
const FLY_MS = 150

function touchDist(t: TouchList): number {
  const dx = t[0].clientX - t[1].clientX
  const dy = t[0].clientY - t[1].clientY
  return Math.sqrt(dx * dx + dy * dy)
}

function flingVelocity(samples: { t: number; x: number; y: number }[]): { vx: number; vy: number } {
  const now = performance.now()
  let start = 0
  while (start < samples.length - 1 && now - samples[start].t > 80) start++
  const first = samples[start]
  const last = samples[samples.length - 1]
  const dt = last.t - first.t
  if (dt <= 0) return { vx: 0, vy: 0 }
  return { vx: (last.x - first.x) / dt, vy: (last.y - first.y) / dt }
}

export function GalleryOverlay(props: {
  items: GalleryItem[]
  index: number
  sourceRect: SourceRect | null
  onClose: () => void
  onPip: (item: GalleryItem, media: HTMLMediaElement) => void
  onNativePip: (media: HTMLMediaElement) => void
  hidden?: boolean
  resume?: { index: number; media: HTMLMediaElement } | null
}) {
  let resume = props.resume ?? null
  const takeResume = (index: number): HTMLMediaElement | undefined => {
    if (!resume || resume.index !== index) return undefined
    const media = resume.media
    resume = null
    return media
  }
  const [cur, setCur] = createSignal(props.index)
  const [thumbs, setThumbs] = createSignal(true)

  let overlay: HTMLDivElement | undefined
  let stage: HTMLDivElement | undefined
  let carousel: HTMLDivElement | undefined
  let track: HTMLDivElement | undefined
  let activeThumb: HTMLButtonElement | undefined
  let activeMedia: HTMLElement | null = null
  let carouselFirst = true

  let transform = { scale: 1, x: 0, y: 0 }
  let pinch: { dist: number; scale: number; tx: number; ty: number; fx: number; fy: number; lfx: number; lfy: number } | null = null
  let drag: {
    x: number
    y: number
    tx: number
    ty: number
    scale: number
    mode: 'pending' | 'pan' | 'swipe' | 'dismiss'
    swipeAnchorX: number
    samples: { t: number; x: number; y: number }[]
  } | null = null
  let tap: { t: number; x: number; y: number } | null = null
  let tapTimer: number | null = null
  let lastTouch = 0
  let settleAnim: Animation | null = null
  let inertiaRaf: number | null = null
  let dragOnStage = false
  let zoomEnabled = true

  const applyTransform = () => {
    const el = activeMedia
    if (!el) return
    const { scale, x, y } = transform
    el.style.transform = `translate(${x}px, ${y}px) scale(${scale})`
  }

  const setTrackDrag = (dx: number) => {
    if (!track) return
    track.style.transition = 'none'
    track.style.transform = `translateX(calc(${-cur() * 100}% + ${dx - cur() * TRACK_GAP}px))`
  }

  const releaseTrack = (navigate: boolean) => {
    if (!track) return
    track.style.transition = ''
    if (!navigate) track.style.transform = `translateX(calc(${-cur() * 100}% - ${cur() * TRACK_GAP}px))`
  }

  const setOverlayDim = (progress: number) => {
    if (!overlay) return
    overlay.style.transition = 'none'
    overlay.style.background = `rgba(0, 0, 0, ${Math.max(0, 0.9 * (1 - Math.min(1, 2.0 * progress)))})`
  }

  const setCarouselDim = (progress: number) => {
    if (!carousel || !thumbs()) return
    carousel.style.transition = 'none'
    carousel.style.opacity = String(Math.max(0, 1 - Math.min(1, 5.0 * progress)))
  }

  const resetTransform = () => {
    if (settleAnim) {
      settleAnim.cancel()
      settleAnim = null
    }
    transform = { scale: 1, x: 0, y: 0 }
    applyTransform()
  }

  const panBounds = (scale: number) => {
    const media = activeMedia
    if (!stage || !media) return { maxX: 0, maxY: 0 }
    return {
      maxX: Math.max(0, (media.offsetWidth * scale - stage.clientWidth) / 2),
      maxY: Math.max(0, (media.offsetHeight * scale - stage.clientHeight) / 2),
    }
  }

  const clampPan = (tx: number, ty: number, scale: number) => {
    const { maxX, maxY } = panBounds(scale)
    return {
      x: Math.min(maxX, Math.max(-maxX, tx)),
      y: Math.min(maxY, Math.max(-maxY, ty)),
    }
  }

  const settleTransform = (px?: number, py?: number) => {
    const { scale, x, y } = transform
    const ns = clamp(scale, 1, 8)
    let tx = x
    let ty = y
    if (px !== undefined && py !== undefined && stage) {
      const rect = stage.getBoundingClientRect()
      const cx = rect.left + rect.width / 2
      const cy = rect.top + rect.height / 2
      const ratio = ns / scale
      tx = px - cx - (px - cx - x) * ratio
      ty = py - cy - (py - cy - y) * ratio
    }
    const c = clampPan(tx, ty, ns)
    if (Math.abs(scale - ns) < 0.001 && Math.abs(x - c.x) < 0.5 && Math.abs(y - c.y) < 0.5) return
    animateTo(ns, c.x, c.y, 220)
  }

  const animateTo = (scale: number, x: number, y: number, duration: number) => {
    const el = activeMedia
    if (!el) return
    const from = transform
    transform = { scale, x, y }
    if (settleAnim) settleAnim.cancel()
    settleAnim = el.animate(
      [
        { transform: `translate(${from.x}px, ${from.y}px) scale(${from.scale})` },
        { transform: `translate(${x}px, ${y}px) scale(${scale})` },
      ],
      { duration, easing: 'ease-out' },
    )
    settleAnim.onfinish = () => {
      settleAnim = null
      applyTransform()
    }
  }

  const startInertia = (vx: number, vy: number) => {
    if (inertiaRaf !== null) cancelAnimationFrame(inertiaRaf)
    const friction = 0.8
    let last = performance.now()
    const step = (now: number) => {
      const dt = Math.min(32, now - last)
      last = now
      const { scale, x, y } = transform
      const nx = x + vx * dt
      const ny = y + vy * dt
      const { maxX, maxY } = panBounds(scale)
      const rx = nx > maxX ? maxX + (nx - maxX) * 0.5 : nx < -maxX ? -maxX + (nx + maxX) * 0.5 : nx
      const ry = ny > maxY ? maxY + (ny - maxY) * 0.5 : ny < -maxY ? -maxY + (ny + maxY) * 0.5 : ny
      transform = { scale, x: rx, y: ry }
      applyTransform()
      const f = Math.pow(friction, dt / 16.7)
      vx *= f
      vy *= f
      if (Math.abs(vx) < 0.05 && Math.abs(vy) < 0.05) {
        inertiaRaf = null
        settleTransform()
      } else {
        inertiaRaf = requestAnimationFrame(step)
      }
    }
    inertiaRaf = requestAnimationFrame(step)
  }

  const doubleTapZoom = (x: number, y: number) => {
    if (!stage || !zoomEnabled) return
    const rect = stage.getBoundingClientRect()
    const cx = rect.left + rect.width / 2
    const cy = rect.top + rect.height / 2
    const current = transform
    if (current.scale > 1) {
      animateTo(1, 0, 0, 250)
      return
    }
    const target = 2.5
    const ratio = target / current.scale
    const tx = x - cx - (x - cx - current.x) * ratio
    const ty = y - cy - (y - cy - current.y) * ratio
    const c = clampPan(tx, ty, target)
    animateTo(target, c.x, c.y, 250)
  }

  const handleTap = (x: number, y: number) => {
    const now = Date.now()
    const last = tap
    if (last && now - last.t < 300 && Math.hypot(x - last.x, y - last.y) < 40) {
      if (tapTimer !== null) {
        window.clearTimeout(tapTimer)
        tapTimer = null
      }
      tap = null
      doubleTapZoom(x, y)
      return
    }
    tap = { t: now, x, y }
    tapTimer = window.setTimeout(() => {
      tapTimer = null
      setThumbs(!thumbs())
    }, 280)
  }

  const select = (i: number) => setCur(i)

  const cancelTransitions = () => {
    if (settleAnim) {
      settleAnim.cancel()
      settleAnim = null
      applyTransform()
    }
    if (inertiaRaf !== null) {
      cancelAnimationFrame(inertiaRaf)
      inertiaRaf = null
      const c = clampPan(transform.x, transform.y, transform.scale)
      transform = { scale: transform.scale, x: c.x, y: c.y }
      applyTransform()
    }
  }

  const beginDrag = (x: number, y: number) => {
    cancelTransitions()
    drag = {
      x,
      y,
      tx: transform.x,
      ty: transform.y,
      scale: transform.scale,
      mode: 'pending',
      swipeAnchorX: x,
      samples: [{ t: performance.now(), x, y }],
    }
  }

  const moveDrag = (x: number, y: number) => {
    const d = drag
    if (!d) return
    const now = performance.now()
    d.samples.push({ t: now, x, y })
    if (d.samples.length > 10) d.samples.shift()

    const dx = x - d.x
    const dy = y - d.y

    if (d.mode === 'pending') {
      const ax = Math.abs(dx)
      const ay = Math.abs(dy)
      if (ax <= 8 && ay <= 8) return
      if (d.scale > 1) d.mode = 'pan'
      else if (ay > ax) d.mode = 'dismiss'
      else {
        d.mode = 'swipe'
        d.swipeAnchorX = d.x
      }
    }

    if (d.mode === 'pan') {
      const { maxX, maxY } = panBounds(d.scale)
      const tx = d.tx + dx
      const ty = d.ty + dy
      const sx = tx > maxX ? maxX + (tx - maxX) * 0.5 : tx < -maxX ? -maxX + (tx + maxX) * 0.5 : tx
      const sy = ty > maxY ? maxY + (ty - maxY) * 0.5 : ty < -maxY ? -maxY + (ty + maxY) * 0.5 : ty
      transform = { scale: d.scale, x: sx, y: sy }
      applyTransform()
      return
    }
    if (d.mode === 'swipe') {
      setTrackDrag(x - d.swipeAnchorX)
      return
    }
    if (d.mode === 'dismiss') {
      if (stage) stage.style.overflow = 'visible'
      const { maxY } = panBounds(d.scale)
      const ty = d.ty + dy
      const cy = Math.min(maxY, Math.max(-maxY, ty))
      const oy = ty - cy
      const shrink = stage ? Math.max(0, oy) / stage.clientHeight : 0
      const newScale = Math.max(0.3, d.scale * (1 - shrink))
      const rect = stage ? stage.getBoundingClientRect() : null
      if (rect) {
        const centerX = rect.left + rect.width / 2
        const centerY = rect.top + rect.height / 2
        const grabX = (d.x - centerX - d.tx) / d.scale
        const grabY = (d.y - centerY - d.ty) / d.scale
        transform = { scale: newScale, x: d.x + dx - centerX - grabX * newScale, y: d.y + dy - centerY - grabY * newScale }
      } else {
        transform = { scale: newScale, x: d.tx + dx, y: d.ty + dy }
      }
      applyTransform()
      const progress = stage ? Math.max(0, oy) / stage.clientHeight : 0
      setOverlayDim(progress)
      setCarouselDim(progress)
    }
  }

  const flyBack = () => {
    const el = activeMedia
    const item = props.items[cur()]
    const target = (item ? findMediaRect(item.thumbUrl) : null) ?? props.sourceRect
    if (!el || !target) {
      props.onClose()
      return
    }
    const r0 = el.getBoundingClientRect()
    if (r0.width <= 0 || r0.height <= 0) {
      props.onClose()
      return
    }
    const { scale, x, y } = transform
    const layoutW = r0.width / scale
    const layoutH = r0.height / scale
    const sTarget = Math.min(target.width / layoutW, target.height / layoutH)
    const ux = r0.left + r0.width / 2 - x
    const uy = r0.top + r0.height / 2 - y
    const txTarget = target.left + target.width / 2 - ux
    const tyTarget = target.top + target.height / 2 - uy
    transform = { scale: sTarget, x: txTarget, y: tyTarget }
    if (overlay) {
      overlay.style.transition = `background ${FLY_MS}ms ease`
      overlay.style.background = 'rgba(0, 0, 0, 0)'
    }
    const bottom = overlay?.querySelector('[data-gallery-bottom]') as HTMLElement | null
    if (bottom) {
      bottom.style.transition = `opacity ${FLY_MS}ms ease`
      bottom.style.opacity = '0'
    }
    const anim = el.animate(
      [
        { transform: `translate(${x}px, ${y}px) scale(${scale})` },
        { transform: `translate(${txTarget}px, ${tyTarget}px) scale(${sTarget})` },
      ],
      { duration: FLY_MS, easing: 'ease-in' },
    )
    anim.onfinish = () => {
      applyTransform()
      props.onClose()
    }
  }

  const endDrag = (x: number, y: number) => {
    const d = drag
    if (!d) return
    const dx = x - d.x
    const dy = y - d.y
    const ax = Math.abs(dx)
    const ay = Math.abs(dy)
    if (stage) stage.style.overflow = 'hidden'
    if (d.mode === 'dismiss' && dy > 0) {
      drag = null
      flyBack()
      return
    }
    if (overlay) {
      overlay.style.transition = ''
      overlay.style.background = ''
    }
    if (carousel) {
      carousel.style.transition = ''
      carousel.style.opacity = ''
    }
    if (d.mode === 'pending') {
      if (ax <= 15 && ay <= 15) {
        if (dragOnStage) props.onClose()
        else handleTap(x, y)
      } else settleTransform()
    } else if (d.mode === 'pan') {
      const { maxX, maxY } = panBounds(d.scale)
      const current = transform
      if (Math.abs(current.x) > maxX + 0.5 || Math.abs(current.y) > maxY + 0.5) settleTransform()
      else {
        const v = flingVelocity(d.samples)
        if (Math.abs(v.vx) > 0.25 || Math.abs(v.vy) > 0.25) startInertia(v.vx, v.vy)
        else settleTransform()
      }
    } else if (d.mode === 'swipe') {
      if (ax > 50) {
        releaseTrack(true)
        const count = props.items.length
        if (dx < 0) select((cur() + 1) % count)
        else select((cur() + count - 1) % count)
      } else {
        releaseTrack(false)
        settleTransform()
      }
    } else if (d.mode === 'dismiss') {
      settleTransform()
    }
    drag = null
  }

  const onStageTouchStart = (e: TouchEvent) => {
    lastTouch = Date.now()
    dragOnStage = e.target === stage
    if (e.touches.length === 1) {
      beginDrag(e.touches[0].clientX, e.touches[0].clientY)
    } else if (e.touches.length === 2 && zoomEnabled) {
      cancelTransitions()
      drag = null
      releaseTrack(false)
      pinch = {
        dist: touchDist(e.touches),
        scale: transform.scale,
        tx: transform.x,
        ty: transform.y,
        fx: (e.touches[0].clientX + e.touches[1].clientX) / 2,
        fy: (e.touches[0].clientY + e.touches[1].clientY) / 2,
        lfx: (e.touches[0].clientX + e.touches[1].clientX) / 2,
        lfy: (e.touches[0].clientY + e.touches[1].clientY) / 2,
      }
    }
  }

  const onStageTouchMove = (e: TouchEvent) => {
    lastTouch = Date.now()
    if (e.touches.length === 2 && pinch) {
      const p = pinch
      if (!stage) return
      let newScale = (p.scale * touchDist(e.touches)) / p.dist
      if (newScale < 1) newScale = 1 - (1 - newScale) * 0.5
      else if (newScale > 8) newScale = 8 + (newScale - 8) * 0.5
      const rect = stage.getBoundingClientRect()
      const cx = rect.left + rect.width / 2
      const cy = rect.top + rect.height / 2
      const fx = (e.touches[0].clientX + e.touches[1].clientX) / 2
      const fy = (e.touches[0].clientY + e.touches[1].clientY) / 2
      p.lfx = fx
      p.lfy = fy
      const ratio = newScale / p.scale
      const tx = fx - cx - (p.fx - cx - p.tx) * ratio
      const ty = fy - cy - (p.fy - cy - p.ty) * ratio
      transform = { scale: newScale, x: tx, y: ty }
      applyTransform()
      return
    }
    if (e.touches.length === 1 && drag) moveDrag(e.touches[0].clientX, e.touches[0].clientY)
  }

  const onStageTouchEnd = (e: TouchEvent) => {
    lastTouch = Date.now()
    if (e.touches.length < 2 && pinch) {
      const lfx = pinch.lfx
      const lfy = pinch.lfy
      pinch = null
      settleTransform(lfx, lfy)
    }
    if (e.touches.length === 0 && drag) endDrag(e.changedTouches[0].clientX, e.changedTouches[0].clientY)
  }

  const onStageMouseDown = (e: MouseEvent) => {
    if (Date.now() - lastTouch < 700) return
    dragOnStage = e.target === stage
    e.preventDefault()
    beginDrag(e.clientX, e.clientY)
    const onMove = (ev: MouseEvent) => moveDrag(ev.clientX, ev.clientY)
    const onUp = (ev: MouseEvent) => {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
      endDrag(ev.clientX, ev.clientY)
    }
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }

  createEffect(() => {
    if (overlay) overlay.inert = props.hidden ?? false
  })

  createEffect(() => {
    document.body.style.overflow = props.hidden ? '' : 'hidden'
    onCleanup(() => {
      document.body.style.overflow = ''
    })
  })

  createEffect(
    on(
      () => cur(),
      (index) => {
        if (!stage) return
        activeMedia = stage.querySelector<HTMLElement>(`[data-index="${index}"]`)
        const item = props.items[index]
        zoomEnabled = !(item && isPlayableMime(item.mime))
        resetTransform()
      },
    ),
  )

  createEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (props.hidden) return
      const count = props.items.length
      if (e.key === 'Escape') {
        props.onClose()
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setCur((c) => (c + count - 1) % count)
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        setCur((c) => (c + 1) % count)
      }
    }
    document.addEventListener('keydown', onKey, { capture: true })
    onCleanup(() => document.removeEventListener('keydown', onKey, { capture: true }))
  })

  createEffect(() => {
    const el = overlay
    if (!el) return
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      if (!stage || !zoomEnabled) return
      const rect = stage.getBoundingClientRect()
      const cx = rect.left + rect.width / 2
      const cy = rect.top + rect.height / 2
      const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12
      const current = transform
      const newScale = clamp(current.scale * factor, 1, 8)
      const ratio = newScale / current.scale
      let tx = e.clientX - cx - (e.clientX - cx - current.x) * ratio
      let ty = e.clientY - cy - (e.clientY - cy - current.y) * ratio
      if (newScale === 1) {
        tx = 0
        ty = 0
      }
      const clamped = clampPan(tx, ty, newScale)
      transform = { scale: newScale, x: clamped.x, y: clamped.y }
      applyTransform()
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    onCleanup(() => el.removeEventListener('wheel', onWheel))
  })

  createEffect(
    on(
      () => cur(),
      () => {
        if (!carousel || !activeThumb) return
        carousel.scrollTo({
          left: activeThumb.offsetLeft - (carousel.clientWidth - activeThumb.offsetWidth) / 2,
          behavior: carouselFirst ? 'auto' : 'smooth',
        })
        carouselFirst = false
      },
    ),
  )

  onCleanup(() => {
    if (inertiaRaf !== null) cancelAnimationFrame(inertiaRaf)
    if (tapTimer !== null) window.clearTimeout(tapTimer)
    if (settleAnim) settleAnim.cancel()
  })

  return (
    <div
      class={galleryOverlayVariants({ hidden: !!props.hidden })}
      ref={overlay}
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose()
      }}
    >
      <div
        class={galleryStageVariants()}
        ref={stage}
        onTouchStart={onStageTouchStart}
        onTouchMove={onStageTouchMove}
        onTouchEnd={onStageTouchEnd}
        onMouseDown={onStageMouseDown}
      >
        <div
          class={galleryTrackVariants()}
          ref={track}
          style={{ transform: `translateX(calc(${-cur() * 100}% - ${cur() * TRACK_GAP}px))` }}
        >
          <For each={props.items}>
            {(m, i) => (
              <div class={gallerySlideVariants()}>
                <Show when={Math.abs(i() - cur()) <= 1}>
                  <Show
                    when={isPlayableMime(m.mime)}
                    fallback={
                      <Show
                        when={m.mime === 'application/pdf'}
                        fallback={
                          <img
                            ref={(el) => {
                              if (i() === cur()) activeMedia = el
                            }}
                            data-index={i()}
                            class={galleryMediaVariants()}
                            src={m.fullUrl}
                            alt=""
                            draggable={false}
                          />
                        }
                      >
                        <a class={galleryPdfCardVariants()} href={m.fullUrl} target="_blank" rel="noreferrer">
                          PDF
                        </a>
                      </Show>
                    }
                  >
                    {(() => {
                      const adopted = takeResume(i())
                      return (
                        <MediaPlayer
                          item={m}
                          index={i()}
                          active={i() === cur()}
                          autoPip={playerPrefs.autoPip() && !adopted}
                          adopt={adopted}
                          rootRef={(el) => {
                            if (i() === cur()) activeMedia = el
                          }}
                          onPip={props.onPip}
                          onNativePip={props.onNativePip}
                          onLeave={props.onClose}
                        />
                      )
                    })()}
                  </Show>
                </Show>
              </div>
            )}
          </For>
        </div>
      </div>
      <div data-gallery-bottom="" class={galleryBottomVariants()}>
        <div ref={carousel} class={galleryThumbsVariants({ visible: thumbs() })}>
          <For each={props.items}>
            {(m, i) => (
              <button
                ref={(el) => {
                  if (i() === cur()) activeThumb = el
                }}
                class={galleryThumbVariants({ active: i() === cur() })}
                onClick={() => select(i())}
              >
                <Show
                  when={m.mime.startsWith('audio/')}
                  fallback={
                    <Show
                      when={m.mime === 'application/pdf'}
                      fallback={<img class={galleryThumbImgVariants()} src={m.thumbUrl} alt="" draggable={false} />}
                    >
                      <span class={galleryThumbPdfVariants()}>PDF</span>
                    </Show>
                  }
                >
                  <span class={galleryThumbAudioVariants()}>
                    <MusicIcon />
                  </span>
                </Show>
                <Show when={m.mime.startsWith('video/')}>
                  <span class={galleryThumbPlayVariants()}>
                    <PlayIcon />
                  </span>
                </Show>
              </button>
            )}
          </For>
        </div>
      </div>
    </div>
  )
}
