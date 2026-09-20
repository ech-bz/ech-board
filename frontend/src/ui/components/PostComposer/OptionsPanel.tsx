import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../../i18n'
import { Field } from '../Field'
import { BbToolbar } from './BbToolbar'
import type { RoleKind, RoleOption } from '../../../core/intent/roles'

const optionsPanelVariants = cva('flex flex-col gap-1.5 rounded-lg bg-glass2 p-2 glass')
const optionsRowVariants = cva('flex flex-wrap items-center gap-1.5')
const checkRowVariants = cva('flex flex-wrap gap-4 px-0.5')
const checkLabelVariants = cva('flex items-center gap-1.5 text-md text-secondary')

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
  onInsertTag: (tag: string) => void
}

export function OptionsPanel(props: OptionsPanelProps) {
  const { t } = useI18n()
  return (
    <div class={optionsPanelVariants()}>
      <Show when={!props.isEdit}>
        <div class={optionsRowVariants()}>
          <Field as="select" grow="min-w-0" value="">
            <option value="">{t('postForm.noHat')}</option>
          </Field>
          <Field as="input" grow="min-w-0" value={props.name} onInput={(e) => props.onNameChange(e.currentTarget.value)} placeholder={t('postForm.name')} />
          <Field as="select" grow="min-w-0" value={props.sendAsKind} onInput={(e) => props.onSendAsKindChange(e.currentTarget.value as RoleKind)}>
            <For each={props.roles}>
              {(option) => (
                <option value={option.kind} title={option.addressHex}>
                  {t(option.labelKey)}
                </option>
              )}
            </For>
          </Field>
          <Show when={props.isThread}>
            <Field as="input" grow="basis-full" value={props.subject} onInput={(e) => props.onSubjectChange(e.currentTarget.value)} placeholder={t('postForm.subject')} />
          </Show>
        </div>
        <div class={checkRowVariants()}>
          <label class={checkLabelVariants()}>
            <input type="checkbox" checked={props.withFlag} onChange={(e) => props.onWithFlagChange(e.currentTarget.checked)} />
            {t('postForm.withFlag')}
          </label>
          <label class={checkLabelVariants()}>
            <input type="checkbox" checked={props.stripExifOn} onChange={(e) => props.onStripExifChange(e.currentTarget.checked)} />
            {t('postForm.stripExif')}
          </label>
          <Show when={props.isThread}>
            <label class={checkLabelVariants()}>
              <input type="checkbox" checked={props.selfAdmin} onChange={(e) => props.onSelfAdminChange(e.currentTarget.checked)} />
              {t('postForm.becomeAdmin')}
            </label>
          </Show>
        </div>
      </Show>
      <BbToolbar onInsertTag={props.onInsertTag} />
    </div>
  )
}
