import { createSignal, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { getSecretKeyBytes, importSecretKey } from '../../core/keys/storage'
import { toHex } from '../../core/intent/crypto'
import { Button } from './Button'
import { Field } from './Field'
import { reportError } from '../errors'

const pageVariants = cva('max-w-settings p-6')
const descVariants = cva('mb-4 text-lg leading-normal text-secondary')
const rowVariants = cva('flex flex-wrap items-center gap-2')
const statusVariants = cva('mt-2.5 text-sm text-secondary')
const importColVariants = cva('mt-4 flex flex-col gap-2.5')
const actionRowVariants = cva('flex items-center gap-2')

export function MasterKeyView() {
  const { t } = useI18n()
  const [importing, setImporting] = createSignal(false)
  const [value, setValue] = createSignal('')
  const [status, setStatus] = createSignal<'' | 'copied' | 'imported' | 'error'>('')
  let masterKey = toHex(getSecretKeyBytes())

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(masterKey)
    } catch {
      const ta = document.createElement('textarea')
      ta.value = masterKey
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      try {
        document.execCommand('copy')
      } catch {
        reportError(t('settings.invalidKey'))
      }
      document.body.removeChild(ta)
    }
    setStatus('copied')
    window.setTimeout(() => setStatus(''), 1200)
  }

  const doImport = () => {
    const hex = value().trim().replace(/^0x/, '')
    try {
      importSecretKey(hex)
      masterKey = toHex(getSecretKeyBytes())
      setImporting(false)
      setValue('')
      setStatus('imported')
      window.setTimeout(() => setStatus(''), 1200)
    } catch {
      setStatus('error')
      window.setTimeout(() => setStatus(''), 1200)
    }
  }

  return (
    <div class={pageVariants()}>
      <p class={descVariants()}>{t('settings.masterKeyDesc')}</p>
      <div class={rowVariants()}>
        <Button onClick={() => void copy()}>{t('settings.copy')}</Button>
        <Button onClick={() => setImporting(true)}>{t('settings.import')}</Button>
      </div>
      <Show when={status()}>
        <div class={statusVariants()}>
          {status() === 'copied' ? t('settings.copied') : status() === 'imported' ? t('settings.imported') : t('settings.invalidKey')}
        </div>
      </Show>
      <Show when={importing()}>
        <div class={importColVariants()}>
          <Field
            as="input"
            type="password"
            bordered
            placeholder={t('settings.importPlaceholder')}
            value={value()}
            autofocus
            onInput={(e) => setValue(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') doImport()
            }}
          />
          <div class={actionRowVariants()}>
            <Button onClick={doImport}>{t('mod.save')}</Button>
            <Button
              onClick={() => {
                setImporting(false)
                setValue('')
              }}
            >
              {t('mod.cancel')}
            </Button>
          </div>
        </div>
      </Show>
    </div>
  )
}
