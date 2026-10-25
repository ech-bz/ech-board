import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../../i18n'
import type { RoleKind, RoleOption } from '../../../core/intent/roles'

const metaStackVariants = cva(
  'grid w-full min-w-0 grid-cols-[auto_minmax(3rem,1fr)_auto] items-center gap-2 min-[900px]:flex min-[900px]:w-auto min-[900px]:flex-row min-[900px]:flex-wrap min-[900px]:justify-end',
)
const chipSelectVariants = cva(
  'select-chevron h-9 min-w-0 max-w-full cursor-pointer appearance-none rounded-full border-0 bg-panel2 px-2 pr-6 text-left text-[15px] text-fg outline-none transition-colors hover:bg-hover min-[900px]:px-3 min-[900px]:pr-8',
)
const chipNameVariants = cva(
  'h-9 w-full min-w-0 rounded-full border-0 bg-panel2 px-2 text-left text-[15px] text-fg outline-none transition-colors placeholder:text-secondary hover:bg-hover min-[900px]:w-[8.25rem] min-[900px]:px-3',
)
const checkRowVariants = cva('flex w-full flex-wrap items-center justify-start gap-x-3 gap-y-2 overflow-visible px-0.5 min-[900px]:w-auto min-[900px]:gap-x-4')
const checkLabelVariants = cva('flex flex-none cursor-pointer items-center gap-1.5 whitespace-nowrap text-[15px] leading-snug text-secondary min-[900px]:gap-2')
const checkboxVariants = cva(
  'post-select-check relative size-[15px] flex-none cursor-pointer appearance-none rounded-[var(--radius-xs)] border border-separator bg-panel2 checked:border-accent checked:bg-accent',
)

export interface OptionsPanelProps {
  isEdit: boolean
  isThread: boolean
  name: string
  onNameChange: (v: string) => void
  sendAsKind: RoleKind
  onSendAsKindChange: (k: RoleKind) => void
  roles: RoleOption[]
  subject: string
  onSubjectChange: (v: string) => void
  withFlag: boolean
  onWithFlagChange: (v: boolean) => void
  stripExifOn: boolean
  onStripExifChange: (v: boolean) => void
  selfAdmin: boolean
  onSelfAdminChange: (v: boolean) => void
  variant?: 'meta' | 'checks'
}

export function OptionsPanel(props: OptionsPanelProps) {
  const { t } = useI18n()
  const variant = () => props.variant ?? 'meta'

  return (
    <Show when={!props.isEdit}>
      <Show when={variant() === 'meta'}>
        <div class={metaStackVariants()}>
          <select class={chipSelectVariants()} value="" aria-label={t('postForm.noHat')}>
            <option value="">{t('postForm.noHat')}</option>
          </select>
          <input
            class={chipNameVariants()}
            value={props.name}
            onInput={(e) => props.onNameChange(e.currentTarget.value)}
            placeholder={t('postForm.name')}
          />
          <select
            class={chipSelectVariants()}
            value={props.sendAsKind}
            onInput={(e) => props.onSendAsKindChange(e.currentTarget.value as RoleKind)}
            aria-label={t('postForm.name')}
          >
            <For each={props.roles}>
              {(option) => (
                <option value={option.kind} title={option.addressHex}>
                  {t(option.labelKey)}
                </option>
              )}
            </For>
          </select>
        </div>
      </Show>
      <Show when={variant() === 'checks'}>
        <div class={checkRowVariants()}>
          <label class={checkLabelVariants()}>
            <input
              type="checkbox"
              class={checkboxVariants()}
              checked={props.withFlag}
              onChange={(e) => props.onWithFlagChange(e.currentTarget.checked)}
            />
            {t('postForm.withFlag')}
          </label>
          <label class={checkLabelVariants()}>
            <input
              type="checkbox"
              class={checkboxVariants()}
              checked={props.stripExifOn}
              onChange={(e) => props.onStripExifChange(e.currentTarget.checked)}
            />
            {t('postForm.stripExif')}
          </label>
          <Show when={props.isThread}>
            <label class={checkLabelVariants()}>
              <input
                type="checkbox"
                class={checkboxVariants()}
                checked={props.selfAdmin}
                onChange={(e) => props.onSelfAdminChange(e.currentTarget.checked)}
              />
              {t('postForm.becomeAdmin')}
            </label>
          </Show>
        </div>
      </Show>
    </Show>
  )
}
