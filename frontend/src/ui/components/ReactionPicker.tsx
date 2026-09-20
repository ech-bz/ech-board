import { createEffect, createSignal, For, onCleanup, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { cva } from 'class-variance-authority'
import { IconButton } from './IconButton'
import { useI18n } from '../i18n'
import { reactionUrl } from '../../core/media'
import { toHex } from '../../core/intent/crypto'

const triggerVariants = cva('post-reactions absolute -bottom-1.5 -right-3 z-control bg-panel3 opacity-50 aria-expanded:z-50 aria-expanded:opacity-100')
const panelVariants = cva('absolute grid w-fit max-w-picker max-h-picker grid-cols-5 overflow-y-auto rounded-lg glass bg-glass2 p-2')
const itemVariants = cva('cursor-pointer rounded-md bg-transparent p-1 hover:bg-reaction')

export function ReactionPicker(props: {
  glyph: string | null
  relayUrl: string
  boardUid: string
  reactions: Uint8Array[]
  busy: boolean
  onPick: (hex: string) => void
}) {
  const { t } = useI18n()
  const [open, setOpen] = createSignal(false)
  const [pos, setPos] = createSignal<{ top: number; right: number } | null>(null)
  let button: HTMLButtonElement | undefined
  let panel: HTMLDivElement | undefined
  let closeTimer: number | null = null
  const canHover = !window.matchMedia('(pointer: coarse)').matches

  const clearCloseTimer = () => {
    if (closeTimer != null) {
      window.clearTimeout(closeTimer)
      closeTimer = null
    }
  }

  const scheduleClose = () => {
    if (closeTimer != null) window.clearTimeout(closeTimer)
    closeTimer = window.setTimeout(() => setOpen(false), 150)
  }

  const openPicker = () => {
    clearCloseTimer()
    setOpen(true)
  }

  createEffect(() => {
    if (!open()) return
    const onDown = (e: MouseEvent | TouchEvent) => {
      const target = e.target as Node
      if (button?.contains(target) || panel?.contains(target)) return
      clearCloseTimer()
      setOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('touchstart', onDown)
    onCleanup(() => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('touchstart', onDown)
    })
  })

  createEffect(() => {
    if (!open() || !panel || !button) return
    const rect = button.getBoundingClientRect()
    const header = document.querySelector('[data-page-header]')
    const limit = header ? header.getBoundingClientRect().bottom : 0
    const height = panel.offsetHeight
    const width = Math.min(panel.offsetWidth || 228, 228)
    const above = rect.top - height >= limit
    const top = Math.max(8, Math.min(above ? rect.top - height : rect.bottom, Math.max(8, window.innerHeight - height - 8)))
    const right = Math.max(8, Math.min(window.innerWidth - 8 - width, window.innerWidth - rect.right))
    setPos({ top: top + window.scrollY, right: right + window.scrollX })
  })

  onCleanup(clearCloseTimer)

  return (
    <Show when={props.glyph}>
      {(glyph) => (
        <>
          <IconButton
            ref={button}
            size="sm"
            data-overhang=""
            class={triggerVariants()}
            aria-label={t('post.reactions')}
            aria-expanded={open()}
            style={{ '--glyph': `url("${glyph()}")` }}
            onMouseEnter={canHover ? openPicker : undefined}
            onMouseLeave={canHover ? scheduleClose : undefined}
            onClick={(e) => {
              e.stopPropagation()
              if (open()) {
                clearCloseTimer()
                setOpen(false)
              } else {
                openPicker()
              }
            }}
          />
          <Show when={open()}>
            <Portal>
              <div
                ref={panel}
                class={panelVariants()}
                style={pos() ? { top: `${pos()!.top}px`, right: `${pos()!.right}px`, 'z-index': 1100 } : { visibility: 'hidden' }}
                role="menu"
                onMouseEnter={canHover ? clearCloseTimer : undefined}
                onMouseLeave={canHover ? scheduleClose : undefined}
              >
                <For each={props.reactions}>
                  {(hash) => {
                    const hex = toHex(hash)
                    return (
                      <button
                        type="button"
                        role="menuitem"
                        class={itemVariants()}
                        disabled={props.busy}
                        onClick={(e) => {
                          e.stopPropagation()
                          clearCloseTimer()
                          setOpen(false)
                          props.onPick(hex)
                        }}
                      >
                        <img src={reactionUrl(props.relayUrl, props.boardUid, hex)} alt="" class="size-5" />
                      </button>
                    )
                  }}
                </For>
              </div>
            </Portal>
          </Show>
        </>
      )}
    </Show>
  )
}
