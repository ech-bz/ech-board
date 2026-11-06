import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { THREAD_SEPARATOR_MAX, THREAD_SEPARATOR_MIN, useI18n, type Lang, type PotatoBoardsBar, type PotatoHeader, type PotatoThreadTitle, type PostBorderSide, type PostReplyPosition, type UiStyle } from '../i18n'
import { haptic } from '../haptic'
import { Field } from './Field'
import { useStaticInfo } from '../store'
import { reactionUrl } from '../../core/media'
import { toHex } from '../../core/intent/crypto'

const pageVariants = cva('box-border w-full max-w-settings p-6')
const settingRowVariants = cva('flex min-w-0 items-center justify-between gap-3 border-b border-separator px-0.5 py-3 text-lg max-[520px]:flex-wrap')
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
    potatoHeader,
    setPotatoHeader,
    potatoBoardsBar,
    setPotatoBoardsBar,
    potatoThreadTitle,
    setPotatoThreadTitle,
    potatoAsciiButtons,
    setPotatoAsciiButtons,
    postBorderSides,
    setPostBorderSides,
    opPostBorders,
    setOpPostBorders,
    opPostBackground,
    setOpPostBackground,
    postReplyPosition,
    setPostReplyPosition,
    threadSeparator,
    setThreadSeparator,
    threadSeparatorHeight,
    setThreadSeparatorHeight,
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
    vistaPalette,
    setVistaPalette,
    vistaBubbles,
    setVistaBubbles,
  } = useI18n()
  const staticInfo = useStaticInfo()
  const borderSideOptions: { id: PostBorderSide; label: string }[] = [
    { id: 'top', label: t('settings.borderTop') },
    { id: 'right', label: t('settings.borderRight') },
    { id: 'bottom', label: t('settings.borderBottom') },
    { id: 'left', label: t('settings.borderLeft') },
  ]
  const toggleBorderSide = (side: PostBorderSide) => {
    const current = postBorderSides()
    setPostBorderSides(current.includes(side) ? current.filter((item) => item !== side) : [...current, side])
  }
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
    <div class={`${pageVariants()} vista-surface`}>
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
      <Show when={uiStyle() !== 'vista'}>
        <div class={settingRowVariants()}>
          <span>{t('settings.theme')}</span>
          <Field as="select" bordered select value={appearance()} onInput={(e) => setAppearance(e.currentTarget.value)}>
            <For each={themes()}>{(theme) => <option value={theme.id}>{theme.label}</option>}</For>
          </Field>
        </div>
      </Show>
      <div class={settingRowVariants()}>
        <span>{t('settings.style')}</span>
        <Field as="select" bordered select value={uiStyle()} onInput={(e) => setUiStyle(e.currentTarget.value as UiStyle)}>
          <option value="modern">Simple</option>
          <option value="potato">Potato</option>
          <option value="vista">Vista</option>
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
        <div class={settingRowVariants()}>
          <span>{t('settings.potatoHeader')}</span>
          <Field as="select" bordered select value={potatoHeader()} onInput={(e) => setPotatoHeader(e.currentTarget.value as PotatoHeader)}>
            <option value="peek">{t('settings.potatoHeaderPeek')}</option>
            <option value="hidden">{t('settings.potatoHeaderHidden')}</option>
            <option value="fixed">{t('settings.potatoHeaderFixed')}</option>
          </Field>
        </div>
        <div class={settingRowVariants()}>
          <span>{t('settings.potatoThreadTitle')}</span>
          <Field as="select" bordered select value={potatoThreadTitle()} onInput={(e) => setPotatoThreadTitle(e.currentTarget.value as PotatoThreadTitle)}>
            <option value="above">{t('settings.potatoThreadTitleAbove')}</option>
            <option value="inline">{t('settings.potatoThreadTitleInline')}</option>
            <option value="hidden">{t('settings.potatoThreadTitleHidden')}</option>
          </Field>
        </div>
        <label class={settingRowVariants()}>
          <span>{t('settings.potatoAsciiButtons')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={potatoAsciiButtons()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setPotatoAsciiButtons(!potatoAsciiButtons())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={potatoAsciiButtons()} readOnly />
          </span>
        </label>
        <Show when={centerPotatoBoards()}>
          <div class={`${settingRowVariants()} max-[899px]:hidden`}>
            <span>{t('settings.potatoBoardsBar')}</span>
            <Field as="select" bordered select value={potatoBoardsBar()} onInput={(e) => setPotatoBoardsBar(e.currentTarget.value as PotatoBoardsBar)}>
              <option value="pinned">{t('settings.potatoBoardsBarPinned')}</option>
              <option value="scroll">{t('settings.potatoBoardsBarScroll')}</option>
            </Field>
          </div>
        </Show>
      </Show>
      <Show when={uiStyle() === 'vista'}>
        <div class={`${settingRowVariants()} max-[520px]:flex-wrap`}>
          <span>{t('settings.vistaPalette')} {vistaPalette()}%</span>
          <Field
            as="input"
            type="range"
            min="0"
            max="100"
            step="5"
            value={vistaPalette()}
            title={`${vistaPalette()}%`}
            aria-label={t('settings.vistaPalette')}
            class="corner-radius-slider w-36 flex-none"
            onInput={(e) => setVistaPalette(Number(e.currentTarget.value))}
          />
        </div>
      </Show>
      <Show when={uiStyle() === 'vista'}>
        <label class={settingRowVariants()}>
          <span>{t('settings.vistaBubbles')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={vistaBubbles()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setVistaBubbles(!vistaBubbles())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={vistaBubbles()} readOnly />
          </span>
        </label>
      </Show>
      <Show when={uiStyle() === 'modern' || uiStyle() === 'potato'}>
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
      <Show when={uiStyle() === 'modern' || uiStyle() === 'potato'}>
        <div class={`${settingRowVariants()} max-[520px]:items-start`}>
          <span>{t('settings.postBorder')}</span>
          <div class="flex flex-wrap items-center justify-end gap-x-3 gap-y-2">
            <For each={borderSideOptions}>
              {(side) => (
                <label class="inline-flex cursor-pointer items-center gap-1.5 text-sm">
                  <input
                    type="checkbox"
                    class="size-4 accent-accent"
                    checked={postBorderSides().includes(side.id)}
                    onChange={() => toggleBorderSide(side.id)}
                  />
                  <span>{side.label}</span>
                </label>
              )}
            </For>
          </div>
        </div>
        <label class={settingRowVariants()}>
          <span>{t('settings.opPostBorders')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={opPostBorders()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setOpPostBorders(!opPostBorders())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={opPostBorders()} readOnly />
          </span>
        </label>
        <label class={settingRowVariants()}>
          <span>{t('settings.opPostBackground')}</span>
          <span
            ref={(el) => haptic(el)}
            role="checkbox"
            aria-checked={opPostBackground()}
            class={toggleWrapVariants()}
            onClick={(e) => {
              e.preventDefault()
              setOpPostBackground(!opPostBackground())
            }}
          >
            <input type="checkbox" class={toggleVariants()} checked={opPostBackground()} readOnly />
          </span>
        </label>
      </Show>
      <label class={settingRowVariants()}>
        <span>{t('settings.threadSeparator')}</span>
        <span
          ref={(el) => haptic(el)}
          role="checkbox"
          aria-checked={threadSeparator()}
          class={toggleWrapVariants()}
          onClick={(e) => {
            e.preventDefault()
            setThreadSeparator(!threadSeparator())
          }}
        >
          <input type="checkbox" class={toggleVariants()} checked={threadSeparator()} readOnly />
        </span>
      </label>
      <Show when={threadSeparator()}>
        <div class={settingRowVariants()}>
          <span>{t('settings.threadSeparatorHeight')} {threadSeparatorHeight()}px</span>
          <Field
            as="input"
            type="range"
            min={String(THREAD_SEPARATOR_MIN)}
            max={String(THREAD_SEPARATOR_MAX)}
            step="1"
            value={threadSeparatorHeight()}
            title={`${threadSeparatorHeight()}px`}
            aria-label={t('settings.threadSeparatorHeight')}
            class="corner-radius-slider w-36 flex-none"
            onInput={(e) => setThreadSeparatorHeight(Number(e.currentTarget.value))}
          />
        </div>
      </Show>
      <div class={settingRowVariants()}>
        <span>{t('settings.replyPosition')}</span>
        <Field
          as="select"
          bordered
          select
          value={postReplyPosition()}
          onInput={(e) => setPostReplyPosition(e.currentTarget.value as PostReplyPosition)}
        >
          <option value="header">{t('settings.replyPositionHeader')}</option>
          <option value="below">{t('settings.replyPositionBelow')}</option>
          <option value="hidden">{t('settings.replyPositionHidden')}</option>
        </Field>
      </div>
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
