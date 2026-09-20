import { type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { Button } from './Button'

const modalScrimVariants = cva('fixed inset-0 z-modal flex items-center justify-center bg-scrim p-6')
const modalPanelVariants = cva('flex max-h-modal w-full max-w-modal flex-col rounded-lg border border-separator bg-panel1')
const modalHeaderVariants = cva('flex items-center justify-between gap-3 border-b border-separator px-4 py-4 text-xl font-bold')
const modalHeaderActionsVariants = cva('flex items-center gap-2')
const modalBodyVariants = cva('min-h-0 overflow-y-auto p-4')

export function Modal(props: { title: string; onClose: () => void; children: JSX.Element; headerActions?: JSX.Element }) {
  const { t } = useI18n()
  return (
    <div class={modalScrimVariants()} onClick={props.onClose}>
      <div class={modalPanelVariants()} onClick={(e) => e.stopPropagation()}>
        <div class={modalHeaderVariants()}>
          <span>{props.title}</span>
          <div class={modalHeaderActionsVariants()}>
            {props.headerActions}
            <Button size="sm" onClick={props.onClose}>
              {t('settings.close')}
            </Button>
          </div>
        </div>
        <div class={modalBodyVariants()}>{props.children}</div>
      </div>
    </div>
  )
}
