import { createEffect, createSignal, onCleanup, type Accessor } from 'solid-js'
import { setUnreadTitle } from './title'

export interface ThreadUnread {
  markerIndex: Accessor<number | null>
  unreadCount: Accessor<number>
  align: () => void
}

export function createThreadUnread(total: Accessor<number>): ThreadUnread {
  const [markerIndex, setMarkerIndex] = createSignal<number | null>(null)
  const [unreadCount, setUnreadCount] = createSignal(0)
  let readCount = 0
  let prevTotal = 0
  let hideTimer: number | null = null
  let scrollRaf: number | null = null
  let wasAtBottom = false

  const atBottom = () => {
    const max = document.documentElement.scrollHeight - window.innerHeight
    return max <= 0 || window.scrollY >= max - 0.5
  }

  const align = () => {
    if (document.visibilityState !== 'visible') return
    wasAtBottom = atBottom()
    if (!wasAtBottom) return
    readCount = total()
    setUnreadCount(0)
    if (hideTimer != null) return
    hideTimer = window.setTimeout(() => {
      hideTimer = null
      setMarkerIndex(null)
    }, 2000)
  }

  const onScroll = () => {
    if (scrollRaf != null) return
    scrollRaf = requestAnimationFrame(() => {
      scrollRaf = null
      align()
    })
  }

  const onVisibility = () => {
    if (document.visibilityState === 'visible') align()
  }

  window.addEventListener('scroll', onScroll, { passive: true })
  document.addEventListener('visibilitychange', onVisibility)

  createEffect(() => {
    const value = total()
    if (value === 0) return
    const previous = prevTotal
    prevTotal = value
    if (previous === 0) {
      readCount = value
      setMarkerIndex(null)
      setUnreadCount(0)
      wasAtBottom = atBottom()
      return
    }
    if (wasAtBottom && document.visibilityState === 'visible') {
      readCount = value
      setUnreadCount(0)
      wasAtBottom = atBottom()
      return
    }
    align()
    if (readCount < value) {
      if (hideTimer != null) {
        window.clearTimeout(hideTimer)
        hideTimer = null
      }
      setMarkerIndex(readCount)
      setUnreadCount(value - readCount)
    }
  })

  createEffect(() => {
    setUnreadTitle(unreadCount())
  })

  onCleanup(() => {
    setUnreadTitle(0)
    window.removeEventListener('scroll', onScroll)
    document.removeEventListener('visibilitychange', onVisibility)
    if (hideTimer != null) window.clearTimeout(hideTimer)
    if (scrollRaf != null) cancelAnimationFrame(scrollRaf)
  })

  return { markerIndex, unreadCount, align }
}
