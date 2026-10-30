import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n, type Lang } from '../i18n'
import { haptic } from '../haptic'
import { Field } from './Field'
import { useStaticInfo } from '../store'
import { reactionUrl } from '../../core/media'
import { toHex } from '../../core/intent/crypto'

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
    hideReactions,
    setHideReactions,
    centerPotatoThreads,
    setCenterPotatoThreads,
    centerPotatoBoards,
    setCenterPotatoBoards,
    showGalleryOnOpen,
    setShowGalleryOnOpen,
    appearance,
    setAppearance,
    uiStyle,
    setUiStyle,
    cornerRadius,
    setCornerRadius,
    reactionSize,
    setReactionSize,
  } = useI18n()
  const staticInfo = useStaticInfo()
  const reactionPreview = () => {
    const info = staticInfo()
    const firstReaction = info?.boardReactions[0]
    return info && firstReaction ? reactionUrl(info.relayUrl, info.boardUid, toHex(firstReaction)) : null
  }
  const themes = () => [
    { id: 'default_dark', label: t('settings.themeDefaultDark') },
    { id: 'default_light', label: t('settings.themeDefaultLight') },
    { id: 'gurochan', label: 'Gurochan' },
    { id: 'neutron', label: 'Neutron' },
    { id: 'jezowy', label: 'Jezowy' },
    { id: 'sosaka_light', label: 'Sosaka A' },
    { id: 'sosaka_dark', label: 'Sosaka B' },
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
        <span>{t('settings.hideReactions')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={hideReactions()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setHideReactions(!hideReactions())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={hideReactions()} readOnly />
        </span>
      </label>
      <label class={settingRowVariants()}>
        <span>{t('settings.showGalleryOnOpen')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={showGalleryOnOpen()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setShowGalleryOnOpen(!showGalleryOnOpen())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={showGalleryOnOpen()} readOnly />
        </span>
      </label>
      <div class={settingRowVariants()}>
        <span>{t('settings.theme')}</span>
        <Field as="select" bordered select value={appearance()} onInput={(e) => setAppearance(e.currentTarget.value)}>
          <For each={themes()}>{(theme) => <option value={theme.id}>{theme.label}</option>}</For>
        </Field>
      </div>
      <div class={settingRowVariants()}>
        <span>{t('settings.style')}</span>
        <Field as="select" bordered select value={uiStyle()} onInput={(e) => setUiStyle(e.currentTarget.value as 'modern' | 'potato')}>
          <option value="modern">Simple</option>
          <option value="potato">Potato</option>
        </Field>
      </div>
      <Show when={uiStyle() === 'potato'}>
        <label class={`${settingRowVariants()} max-[899px]:hidden`}>
          <span>{t('settings.centerPotatoThreads')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={centerPotatoThreads()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setCenterPotatoThreads(!centerPotatoThreads())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={centerPotatoThreads()} readOnly />
          </span>
        </label>
        <label class={`${settingRowVariants()} max-[899px]:hidden`}>
          <span>{t('settings.centerPotatoBoards')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={centerPotatoBoards()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setCenterPotatoBoards(!centerPotatoBoards())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={centerPotatoBoards()} readOnly />
          </span>
        </label>
      </Show>
      <Show when={uiStyle() === 'modern'}>
        <div class={settingRowVariants()}>
          <span>{t('settings.radius')} {cornerRadius()}%</span>
          <Field
            as="input"
            type="range"
            min="0"
            max="100"
            step="10"
            value={cornerRadius()}
            title={`${cornerRadius()}%`}
            aria-label={t('settings.radius')}
            class="corner-radius-slider w-36 flex-none"
            onInput={(e) => setCornerRadius(Number(e.currentTarget.value))}
          />
        </div>
      </Show>
      <div class={`${settingRowVariants()} max-[420px]:flex-wrap`}>
        <span>{t('settings.reactionSize')} {reactionSize()}%</span>
        <div class="ml-auto flex flex-none items-center gap-3">
          <Show when={reactionPreview()}>
            {(src) => (
              <div class="flex size-10 flex-none items-center justify-center rounded-md border border-separator bg-panel1" aria-label={t('settings.reactionSize')}>
                <img src={src()} alt={t('settings.reactionSize')} class="reaction-image" />
              </div>
            )}
          </Show>
          <Field
            as="input"
            type="range"
            min="0"
            max="100"
            step="10"
            value={reactionSize()}
            title={`${reactionSize()}%`}
            aria-label={t('settings.reactionSize')}
            class="corner-radius-slider w-36 flex-none"
            onInput={(e) => setReactionSize(Number(e.currentTarget.value))}
          />
        </div>
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
