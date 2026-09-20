import { cva } from 'class-variance-authority'
import { Show } from 'solid-js'
import { useI18n } from '../i18n'

const wrapVariants = cva('flex min-h-dvh items-center justify-center p-8 text-secondary')

export function StateScreen(props: { error?: string | null }) {
  const { t } = useI18n()
  return (
    <div class={wrapVariants()}>
      <Show when={props.error} fallback={t('ui.loading')}>
        {(message) => <>{message()}</>}
      </Show>
    </div>
  )
}
