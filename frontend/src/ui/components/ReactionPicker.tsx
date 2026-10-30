import { createEffect, createSignal, For, onCleanup, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { cva } from 'class-variance-authority'
import { IconButton } from './IconButton'
import { useI18n, useIsPotato } from '../i18n'
import { reactionUrl } from '../../core/media'
import { toHex } from '../../core/intent/crypto'

const triggerVariants = cva('post-reactions reaction-trigger z-control inline-flex items-center justify-center rounded-full border-0 bg-transparent p-0 opacity-100 aria-expanded:z-50 aria-expanded:opacity-100 hover:bg-transparent', {
  variants: {
    potato: {
      true: 'absolute right-0 top-1/2 z-10 !size-6 !min-h-6 !min-w-6 flex-none rounded-full !bg-transparent !p-0 opacity-0 pointer-events-none -translate-y-1/2',
      false: 'absolute -bottom-1.5 -right-3',
    },
  },
  defaultVariants: { potato: false },
})
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
  const potato = useIsPotato()
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
            class={triggerVariants({ potato: potato() })}
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
                        <img src={reactionUrl(props.relayUrl, props.boardUid, hex)} alt="" class="reaction-picker-image" />
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
