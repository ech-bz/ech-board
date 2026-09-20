import { createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import type { MessageKey } from '../i18n/ru'
import { Modal } from './Modal'
import { toHex } from '../../core/intent/crypto'
import type { ModeratorsData } from '../../core/bcs/types'

const groupVariants = cva('mb-4 last:mb-0')
const groupLabelVariants = cva('mb-1.5 font-bold')
const listVariants = cva('flex flex-col gap-1.5')
const listItemVariants = cva('flex cursor-pointer select-all items-center justify-between gap-3 rounded-md bg-panel2 px-3 py-2 font-mono text-sm break-all text-secondary hover:bg-hover')
const copiedVariants = cva('flex-none text-xs text-accent')
const emptyVariants = cva('text-secondary')

interface Group {
  labelKey: MessageKey
  items: Uint8Array[]
}

export function ModeratorsModal(props: { moderators: ModeratorsData; onClose: () => void }) {
  const { t } = useI18n()
  const [copied, setCopied] = createSignal<string | null>(null)
  const groups = (): Group[] => {
    const result: Group[] = []
    const mods = props.moderators
    if (mods.forum_admin) result.push({ labelKey: 'mods.forumAdmin', items: [mods.forum_admin] })
    if (mods.forum_mods.length) result.push({ labelKey: 'mods.forumMods', items: mods.forum_mods })
    if (mods.board_mods.length) result.push({ labelKey: 'mods.boardMods', items: mods.board_mods })
    if (mods.thread_admin) result.push({ labelKey: 'mods.threadAdmin', items: [mods.thread_admin] })
    if (mods.thread_mods.length) result.push({ labelKey: 'mods.threadMods', items: mods.thread_mods })
    return result
  }
  const copy = async (hex: string) => {
    try {
      await navigator.clipboard.writeText(hex)
    } catch {
      setCopied(hex)
      window.setTimeout(() => setCopied(null), 1200)
      return
    }
    setCopied(hex)
    window.setTimeout(() => setCopied(null), 1200)
  }
  return (
    <Modal title={t('mod.moderators')} onClose={props.onClose}>
      <Show when={groups().length > 0} fallback={<p class={emptyVariants()}>{t('mods.empty')}</p>}>
        <For each={groups()}>
          {(group) => (
            <div class={groupVariants()}>
              <div class={groupLabelVariants()}>{t(group.labelKey)}</div>
              <div class={listVariants()}>
                <For each={group.items}>
                  {(address) => (
                    <div class={listItemVariants()} title={t('settings.copy')} onClick={() => void copy(toHex(address))}>
                      <span>{toHex(address)}</span>
                      <Show when={copied() === toHex(address)}>
                        <span class={copiedVariants()}>{t('settings.copied')}</span>
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            </div>
          )}
        </For>
      </Show>
    </Modal>
  )
}
