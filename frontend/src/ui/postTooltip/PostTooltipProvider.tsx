import { createSignal, onCleanup, Show, For, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import { Ctx, type Api, type PostTooltipTarget, type ResolvePost } from './usePostTooltip'
import { TooltipLayer, type Entry } from './TooltipLayer'
import type { TooltipPost } from '../../core/load'

let nextId = 1
const LEAVE_DELAY = 500

export function PostTooltipProvider(props: {
  resolve: ResolvePost
  boardPage?: boolean
  children: JSX.Element
}) {
  const [entries, setEntries] = createSignal<Entry[]>([])
  let current: Entry[] = []
  let closeTimer: number | null = null
  let lastMove: PointerEvent | null = null
  const elements = new Map<number, HTMLElement>()

  const update = (next: Entry[]) => {
    current = next
    setEntries(next)
  }

  const clearClose = () => {
    if (closeTimer != null) {
      window.clearTimeout(closeTimer)
      closeTimer = null
    }
  }

  const closeAll = () => {
    clearClose()
    elements.clear()
    update([])
  }

  const open = (anchor: HTMLElement, target: PostTooltipTarget) => {
    if (current.some((entry) => entry.anchor === anchor)) return
    clearClose()
    let nested = false
    for (let i = current.length - 1; i >= 0; i--) {
      const el = elements.get(current[i].id)
      if (el && el.contains(anchor)) {
        nested = true
        break
      }
    }
    const [result, setResult] = createSignal<TooltipPost | null>(null)
    const [error, setError] = createSignal(false)
    const entry: Entry = { id: nextId++, depth: nested ? current.length : 0, anchor, target, result, error }
    update(nested ? [...current, entry] : [entry])
    props
      .resolve(target.postUid)
      .then((value) => {
        setResult(value)
        setError(!value)
      })
      .catch(() => setError(true))
  }

  const isOpen = (anchor: HTMLElement) => current.some((entry) => entry.anchor === anchor)
  const api: Api = { open, isOpen }

  const isOver = (el: HTMLElement | null | undefined, e: PointerEvent): boolean => {
    if (!el) return false
    if (el.contains(e.target as Node)) return true
    const r = el.getBoundingClientRect()
    const pad = 8
    return e.clientX >= r.left - pad && e.clientX <= r.right + pad && e.clientY >= r.top - pad && e.clientY <= r.bottom + pad
  }

  const deepestUnder = (e: PointerEvent): number => {
    let deeperKept = false
    let deepestKept = -1
    for (let i = current.length - 1; i >= 0; i--) {
      const inOwn = isOver(elements.get(current[i].id), e) || isOver(current[i].anchor, e)
      const stay: boolean = inOwn || deeperKept
      if (stay && deepestKept === -1) deepestKept = i
      deeperKept = stay
    }
    return deepestKept
  }

  const keepUnder = (e: PointerEvent, delayed: boolean): boolean => {
    if (current.length === 0) {
      clearClose()
      return false
    }
    const deepestKept = deepestUnder(e)
    if (deepestKept === -1) {
      if (!delayed) {
        closeAll()
        return false
      }
      if (closeTimer == null) {
        closeTimer = window.setTimeout(() => {
          closeTimer = null
          if (lastMove && deepestUnder(lastMove) !== -1) return
          closeAll()
        }, LEAVE_DELAY)
      }
      return true
    }
    clearClose()
    if (deepestKept < current.length - 1) {
      for (let i = deepestKept + 1; i < current.length; i++) elements.delete(current[i].id)
      update(current.slice(0, deepestKept + 1))
    }
    return true
  }

  const onMove = (e: PointerEvent) => {
    lastMove = e
    keepUnder(e, true)
  }
  const onDown = (e: PointerEvent) => {
    lastMove = e
    keepUnder(e, false)
  }

  document.addEventListener('pointermove', onMove, true)
  document.addEventListener('pointerdown', onDown, true)
  onCleanup(() => {
    clearClose()
    document.removeEventListener('pointermove', onMove, true)
    document.removeEventListener('pointerdown', onDown, true)
  })

  return (
    <Ctx.Provider value={api}>
      {props.children}
      <Show when={entries().length > 0}>
        <Portal>
          <For each={entries()}>
            {(entry) => (
              <TooltipLayer
                entry={entry}
                boardPage={props.boardPage}
                register={(el) => {
                  if (el) elements.set(entry.id, el)
                  else elements.delete(entry.id)
                }}
              />
            )}
          </For>
        </Portal>
      </Show>
    </Ctx.Provider>
  )
}
