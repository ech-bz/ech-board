import { createEffect, createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Modal } from './Modal'
import { Button } from './Button'
import { fetchBans } from '../../core/api/relay'
import { BansView } from '../../core/bcs/types'
import { toHex } from '../../core/intent/crypto'
import { useI18n } from '../i18n'

const errorVariants = cva('my-1 text-md text-danger')
const emptyVariants = cva('my-1 text-md text-secondary')
const tableVariants = cva('w-full border-collapse')
const thVariants = cva('border-b border-separator px-2.5 py-2 text-left align-top font-semibold break-all')
const tdVariants = cva('border-b border-separator px-2.5 py-2 text-left align-top break-all')

interface BanRow {
  mask: number
  ip_hash: string
  reason: string | null
  expires: number
}

export function BansModal(props: { title: string; relayUrl: string; uid: string; onClose: () => void }) {
  const { t } = useI18n()
  const [bans, setBans] = createSignal<BanRow[]>([])
  const [nextCursor, setNextCursor] = createSignal<number | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)

  const load = async (cursor?: number) => {
    if (!props.relayUrl || !props.uid) return
    setLoading(true)
    setError(null)
    try {
      const bytes = await fetchBans(props.relayUrl, props.uid, cursor)
      const view = BansView.parse(new Uint8Array(bytes))
      const list = (view.bans as unknown as { mask: number; ip_hash: Uint8Array; reason: string | null; expires: number }[]).map(
        (ban) => ({
          mask: Number(ban.mask),
          ip_hash: toHex(ban.ip_hash),
          reason: ban.reason,
          expires: Number(ban.expires),
        }),
      )
      setBans(cursor ? [...bans(), ...list] : list)
      setNextCursor(view.next_cursor != null ? Number(view.next_cursor) : null)
    } catch (e) {
      setError(String(e))
    }
    setLoading(false)
  }

  createEffect(() => {
    if (!props.relayUrl || !props.uid) return
    setBans([])
    setNextCursor(null)
    setError(null)
    void load()
  })

  return (
    <Modal title={props.title} onClose={props.onClose}>
      <Show when={error()}>{(message) => <div class={errorVariants()}>{message()}</div>}</Show>
      <Show when={bans().length === 0 && !loading()}>
        <p class={emptyVariants()}>{t('bans.empty')}</p>
      </Show>
      <Show when={bans().length > 0}>
        <table class={tableVariants()}>
          <thead>
            <tr>
              <th class={thVariants()}>{t('bans.mask')}</th>
              <th class={thVariants()}>{t('bans.ip')}</th>
              <th class={thVariants()}>{t('bans.reason')}</th>
              <th class={thVariants()}>{t('bans.expires')}</th>
            </tr>
          </thead>
          <tbody>
            <For each={bans()}>
              {(ban) => (
                <tr>
                  <td class={tdVariants()}>/{ban.mask}</td>
                  <td class={tdVariants()}>{ban.ip_hash}</td>
                  <td class={tdVariants()}>{ban.reason || t('bans.noData')}</td>
                  <td class={tdVariants()}>{new Date(ban.expires).toLocaleString()}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={nextCursor() != null}>
        <Button disabled={loading()} onClick={() => void load(nextCursor()!)}>
          {loading() ? t('common.loading') : t('bans.loadMore')}
        </Button>
      </Show>
    </Modal>
  )
}
