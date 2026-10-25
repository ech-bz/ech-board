import { createEffect, createSignal, on, onCleanup } from 'solid-js'

const OPEN_MS = 240
const CLOSE_MS = 10
const EASING = 'cubic-bezier(0.2, 0, 0, 1)'

export function createOptionsReveal(expanded: () => boolean) {
  const [shown, setShown] = createSignal(expanded())
  let box: HTMLElement | undefined
  let options: HTMLElement | undefined
  let above: HTMLElement | undefined
  let running: Animation[] = []

  const promote = (on: boolean) => {
    if (box) box.style.willChange = on ? 'clip-path' : ''
    if (options) options.style.willChange = on ? 'opacity' : ''
    if (above) above.style.willChange = on ? 'transform' : ''
  }

  const stop = () => {
    for (const animation of running) animation.cancel()
    running = []
    promote(false)
  }
  onCleanup(stop)

  const play = (delta: number, radius: string, out: boolean) => {
    const clip = (px: number) => `inset(${px}px 0px 0px 0px round ${radius})`
    promote(true)
    const timing: KeyframeAnimationOptions = { duration: out ? CLOSE_MS : OPEN_MS, easing: EASING, fill: out ? 'forwards' : 'none' }
    const list: Animation[] = [
      box!.animate({ clipPath: out ? [clip(0), clip(delta)] : [clip(delta), clip(0)] }, timing),
      options!.animate({ opacity: out ? [1, 0] : [0, 1] }, timing),
    ]
    if (above) list.push(above.animate({ transform: out ? ['translateY(0)', `translateY(${delta}px)`] : [`translateY(${delta}px)`, 'translateY(0)'] }, timing))
    running.push(...list)
    const finished = list[0].finished
    if (!out) finished.then(() => promote(false)).catch(() => undefined)
    return finished
  }

  createEffect(
    on(
      expanded,
      (open) => {
        stop()
        const canAnimate = box && options && typeof box.animate === 'function' && !window.matchMedia('(prefers-reduced-motion: reduce)').matches
        if (!canAnimate) {
          setShown(open)
          return
        }
        const style = getComputedStyle(box!)
        const radius = style.borderTopLeftRadius
        const gap = Number.parseFloat(style.rowGap) || 0
        if (open) {
          setShown(true)
          play(options!.offsetHeight + gap, radius, false).catch(() => undefined)
          return
        }
        play(options!.offsetHeight + gap, radius, true)
          .then(() => {
            setShown(false)
            stop()
          })
          .catch(() => undefined)
      },
      { defer: true },
    ),
  )

  return {
    shown,
    setBox: (el: HTMLElement) => (box = el),
    setOptions: (el: HTMLElement) => (options = el),
    setAbove: (el: HTMLElement) => (above = el),
  }
}
