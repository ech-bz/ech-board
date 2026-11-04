import { createEffect, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { useI18n, useIsVista } from '../i18n'

type Bubble = {
  id: number
  left: number
  size: number
  duration: number
  drift: number
  popped: boolean
}

export function VistaBubbles() {
  const vista = useIsVista()
  const { t, vistaBubbles } = useI18n()
  const [bubbles, setBubbles] = createSignal<Bubble[]>([])
  let layer: HTMLDivElement | undefined
  let nextId = 0
  const removalTimers = new Map<number, number>()

  const remove = (id: number) => {
    const timer = removalTimers.get(id)
    if (timer != null) window.clearTimeout(timer)
    removalTimers.delete(id)
    setBubbles((items) => items.filter((item) => item.id !== id))
  }

  const pop = (id: number) => {
    setBubbles((items) => items.map((item) => item.id === id ? { ...item, popped: true } : item))
    const old = removalTimers.get(id)
    if (old != null) window.clearTimeout(old)
    removalTimers.set(id, window.setTimeout(() => remove(id), 240))
  }

  const spawn = () => {
    const amount = Math.random() < 0.38 ? 2 : 1
    const added: Bubble[] = Array.from({ length: amount }, () => {
      const id = ++nextId
      const bubble: Bubble = {
        id,
        left: 5 + Math.random() * 90,
        size: 58 + Math.round(Math.random() * 72),
        duration: 8 + Math.random() * 4,
        drift: -38 + Math.random() * 76,
        popped: false,
      }
      removalTimers.set(id, window.setTimeout(() => remove(id), bubble.duration * 1000 + 250))
      return bubble
    })
    setBubbles((items) => [...items.filter((item) => !item.popped).slice(-3), ...added])
  }

  createEffect(() => {
    if (!vista() || !vistaBubbles()) {
      setBubbles([])
      for (const timer of removalTimers.values()) window.clearTimeout(timer)
      removalTimers.clear()
      return
    }
    const firstTimer = window.setTimeout(spawn, 1600)
    const intervalTimer = window.setInterval(spawn, 4800)
    onCleanup(() => {
      window.clearTimeout(firstTimer)
      window.clearInterval(intervalTimer)
      for (const timer of removalTimers.values()) window.clearTimeout(timer)
      removalTimers.clear()
    })
  })

  onMount(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (!vista() || !layer) return
      const target = event.target as HTMLElement | null
      const bubble = target?.closest<HTMLElement>('[data-vista-bubble]')
      if (bubble) {
        pop(Number(bubble.dataset.vistaBubble))
        return
      }
      if (target?.closest('button, a, input, textarea, select, [role="button"], [data-vista-window], .vista-surface, [data-post], [data-composer], [data-page-header], [data-vista-bubble-ignore]')) return
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    onCleanup(() => document.removeEventListener('pointerdown', onPointerDown, true))
  })

  return (
    <Show when={vista() && vistaBubbles()}>
      <div ref={layer} class="vista-bubble-layer">
        <For each={bubbles()}>
          {(bubble) => (
            <span
              data-vista-bubble={bubble.id}
              class={`vista-bubble${bubble.popped ? ' is-popped' : ''}`}
              role="button"
              tabIndex={0}
              aria-label={t('vista.popBubble')}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  pop(bubble.id)
                }
              }}
              style={`left:${bubble.left}%;--bubble-size:${bubble.size}px;--bubble-duration:${bubble.duration}s;--bubble-drift:${bubble.drift}px`}
            />
          )}
        </For>
      </div>
    </Show>
  )
}
