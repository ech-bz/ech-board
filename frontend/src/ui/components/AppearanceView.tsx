import { For } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n, type Lang } from '../i18n'
import { haptic } from '../haptic'
import { Field } from './Field'
import { playerPrefs } from './Player/playerPrefs'

const pageVariants = cva('max-w-settings p-6')
const settingRowVariants = cva('flex items-center justify-between gap-3 border-b border-separator px-0.5 py-3 text-lg')
const toggleWrapVariants = cva('inline-flex flex-none cursor-pointer')
const toggleVariants = cva('settings-checkbox relative h-toggle w-toggle flex-none cursor-pointer appearance-none rounded-full bg-toggle-track checked:bg-toggle-on transition-colors')

export function AppearanceView() {
  const {
    t,
    lang,
    setLang,
    showDeleted,
    setShowDeleted,
    hideThreadsCompletely,
    setHideThreadsCompletely,
    expandFolds,
    setExpandFolds,
    appearance,
    setAppearance,
  } = useI18n()
  const themes = () => [
    { id: 'default_dark', label: t('settings.themeDefaultDark') },
    { id: 'default_light', label: t('settings.themeDefaultLight') },
    { id: 'gurochan', label: 'Gurochan' },
    { id: 'neutron', label: 'Neutron' },
    { id: 'dark_oled', label: 'Dark OLED' },
  ]
  return (
    <div class={pageVariants()}>
      <label class={settingRowVariants()}>
        <span>{t('settings.showDeleted')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={showDeleted()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setShowDeleted(!showDeleted())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={showDeleted()} readOnly />
        </span>
      </label>
      <label class={settingRowVariants()}>
        <span>{t('settings.showHidden')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={!hideThreadsCompletely()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setHideThreadsCompletely(!hideThreadsCompletely())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={!hideThreadsCompletely()} readOnly />
        </span>
      </label>
      <label class={settingRowVariants()}>
        <span>{t('settings.expandFolds')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={expandFolds()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setExpandFolds(!expandFolds())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={expandFolds()} readOnly />
        </span>
      </label>
      <label class={settingRowVariants()}>
        <span>{t('settings.autoPip')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={playerPrefs.autoPip()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            playerPrefs.setAutoPip(!playerPrefs.autoPip())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={playerPrefs.autoPip()} readOnly />
        </span>
      </label>
      <div class={settingRowVariants()}>
        <span>{t('settings.theme')}</span>
        <Field as="select" bordered select value={appearance()} onInput={(e) => setAppearance(e.currentTarget.value)}>
          <For each={themes()}>{(theme) => <option value={theme.id}>{theme.label}</option>}</For>
        </Field>
      </div>
      <div class={settingRowVariants()}>
        <span>{t('settings.language')}</span>
        <Field as="select" bordered select value={lang()} onInput={(e) => setLang(e.currentTarget.value as Lang)}>
          <option value="ru">Русский</option>
          <option value="en">English</option>
        </Field>
      </div>
    </div>
  )
}
