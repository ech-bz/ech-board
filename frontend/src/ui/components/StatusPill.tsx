import { Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'

const badgeVariants = cva('pointer-events-none fixed left-1/2 top-[calc(76px+env(safe-area-inset-top,0px))] z-30 flex -translate-x-1/2 items-center gap-2 rounded-full bg-panel3 px-3 py-1.5')
const spinnerVariants = cva('size-3 animate-spin rounded-full border-2 border-track border-t-accent')

export function StatusPill(props: { reconnecting: boolean; refreshing: boolean }) {
  const { t } = useI18n()
  return (
    <>
      <Show when={props.reconnecting}>
        <div class={badgeVariants()} role="status">
          <span class={spinnerVariants()} />
          <span>{t('ui.reconnecting')}</span>
        </div>
      </Show>
      <Show when={props.refreshing}>
        <div class={badgeVariants()} role="status">
          <span class={spinnerVariants()} />
          <span>{t('ui.refreshing')}</span>
        </div>
      </Show>
    </>
  )
}
