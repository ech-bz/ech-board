import { createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import type { MessageKey } from '../i18n/ru'
import { Button } from './Button'
import { Field } from './Field'
import { IconButton } from './IconButton'
import { XIcon } from '../icons/XIcon'

const scrimVariants = cva('fixed inset-0 z-modal flex items-center justify-center bg-scrim p-6')
const panelVariants = cva('flex max-h-modal w-full max-w-[420px] flex-col rounded-lg border border-separator bg-panel1 p-5')
const headerVariants = cva('mb-4 flex flex-none items-center justify-between gap-3')
const titleVariants = cva('text-2xl font-semibold leading-tight')
const bodyVariants = cva('flex min-h-0 flex-col gap-3 overflow-y-auto')
const reasonVariants = cva(
  'w-full cursor-pointer rounded-full border-0 bg-panel3 px-5 py-3 text-center text-lg font-medium leading-snug text-fg text-balance hover:bg-hover',
)
const otherVariants = cva('flex flex-col gap-3')

const REASONS: { id: string; key: MessageKey }[] = [
  { id: 'minors', key: 'report.reason.minors' },
  { id: 'terror', key: 'report.reason.terror' },
  { id: 'doxing', key: 'report.reason.doxing' },
  { id: 'violence', key: 'report.reason.violence' },
  { id: 'drugs', key: 'report.reason.drugs' },
  { id: 'spam', key: 'report.reason.spam' },
  { id: 'ads', key: 'report.reason.ads' },
]

export function ReportModal(props: {
  onClose: () => void
  onSubmit: (reason: string) => void
}) {
  const { t } = useI18n()
  const [other, setOther] = createSignal(false)
  const [text, setText] = createSignal('')
  const send = () => {
    const value = text().trim()
    if (!value) return
    props.onSubmit(value)
    props.onClose()
  }
  const pick = (key: MessageKey) => {
    props.onSubmit(t(key))
    props.onClose()
  }
  return (
    <div class={scrimVariants()} onClick={props.onClose}>
      <div class={panelVariants()} role="dialog" aria-modal="true" aria-label={t('report.title')} onClick={(e) => e.stopPropagation()}>
        <div class={headerVariants()}>
          <span class={titleVariants()}>{t('report.title')}</span>
          <IconButton size="md" class="hover:bg-hover" aria-label={t('report.close')} title={t('report.close')} onClick={props.onClose}>
            <XIcon width={22} height={22} />
          </IconButton>
        </div>
        <Show
          when={!other()}
          fallback={
            <div class={otherVariants()}>
              <Field
                type="text"
                bordered
                maxLength={500}
                placeholder={t('report.otherPlaceholder')}
                aria-label={t('report.otherPlaceholder')}
                value={text()}
                onInput={(e) => setText(e.currentTarget.value)}
                onKeyDown={(e) => e.key === 'Enter' && send()}
                ref={(el: HTMLInputElement) => queueMicrotask(() => el.focus())}
              />
              <Button class="self-end" disabled={!text().trim()} onClick={send}>
                {t('report.send')}
              </Button>
            </div>
          }
        >
          <div class={bodyVariants()}>
            <For each={REASONS}>
              {(reason) => (
                <button type="button" class={reasonVariants()} onClick={() => pick(reason.key)}>
                  {t(reason.key)}
                </button>
              )}
            </For>
            <button type="button" class={reasonVariants()} onClick={() => setOther(true)}>
              {t('report.reason.other')}
            </button>
          </div>
        </Show>
      </div>
    </div>
  )
}
