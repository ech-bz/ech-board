import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { THREAD_SEPARATOR_MAX, THREAD_SEPARATOR_MIN, useI18n, type Lang, type PotatoBoardsBar, type PotatoHeader, type PotatoThreadTitle, type PostBorderSide, type PostReplyPosition, type UiStyle } from '../i18n'
import { haptic } from '../haptic'
import { Field } from './Field'
import { Fold } from './Fold'
import { useStaticInfo } from '../store'
import { reactionUrl } from '../../core/media'
import { toHex } from '../../core/intent/crypto'

const pageVariants = cva('box-border w-full max-w-settings p-6')
const settingRowVariants = cva('flex min-w-0 items-center justify-between gap-3 border-b border-separator px-0.5 py-3 text-lg last:border-b-0 max-[520px]:flex-wrap')
const toggleWrapVariants = cva('inline-flex flex-none cursor-pointer')
const toggleVariants = cva('settings-checkbox relative h-toggle w-toggle flex-none cursor-pointer appearance-none rounded-full bg-toggle-track checked:bg-toggle-on transition-colors')

function ToggleRow(props: { label: string; checked: boolean; onToggle: () => void; class?: string }) {
  return (
    <label class={`${settingRowVariants()} ${props.class ?? ''}`}>
      <span>{props.label}</span>
      <span
        ref={(el) => haptic(el)}
        role="checkbox"
        aria-checked={props.checked}
        class={toggleWrapVariants()}
        onClick={(e) => {
          e.preventDefault()
          props.onToggle()
        }}
      >
        <input type="checkbox" class={toggleVariants()} checked={props.checked} readOnly />
      </span>
    </label>
  )
}

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
    preset,
    setPreset,
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
    { id: 'yotsuba_a', label: 'Yotsuba A' },
    { id: 'yotsuba_b', label: 'Yotsuba B' },
    { id: 'gurochan', label: 'Gurochan' },
    { id: 'nowak', label: 'Nowak' },
    { id: 'neutron', label: 'Neutron' },
    { id: 'jezowy', label: 'Jezowy' },
    { id: 'sosaka_light', label: 'Sosaka A' },
    { id: 'sosaka_dark', label: 'Sosaka B' },
    { id: 'dark_oled', label: 'Dark OLED' },
  ]
  const styleName = () => (uiStyle() === 'potato' ? 'Potato' : uiStyle() === 'vista' ? 'Vista' : 'Simple')
  const hasStyleOptions = () => uiStyle() === 'potato' || uiStyle() === 'vista'
  const hasAdvanced = () => uiStyle() === 'modern' || uiStyle() === 'potato'
  return (
    <div class={`${pageVariants()} vista-surface`}>
      <Fold title={t('settings.groupGeneral')} defaultOpen followExpandFolds={false}>
        <div class={settingRowVariants()}>
          <span>{t('settings.language')}</span>
          <Field as="select" bordered select value={lang()} onInput={(e) => setLang(e.currentTarget.value as Lang)}>
            <option value="ru">Русский</option>
            <option value="en">English</option>
          </Field>
        </div>
        <ToggleRow label={t('settings.showDeleted')} checked={showDeleted()} onToggle={() => setShowDeleted(!showDeleted())} />
        <ToggleRow label={t('settings.showHidden')} checked={!hideThreadsCompletely()} onToggle={() => setHideThreadsCompletely(!hideThreadsCompletely())} />
        <ToggleRow label={t('settings.expandFolds')} checked={expandFolds()} onToggle={() => setExpandFolds(!expandFolds())} />
        <ToggleRow label={t('settings.showGalleryOnOpen')} checked={showGalleryOnOpen()} onToggle={() => setShowGalleryOnOpen(!showGalleryOnOpen())} />
      </Fold>
      <Fold title={t('settings.groupAppearance')} defaultOpen followExpandFolds={false}>
        <div class={settingRowVariants()}>
          <span>{t('settings.presets')}</span>
          <Field as="select" bordered select value={preset()} onInput={(e) => setPreset(e.currentTarget.value as 'default' | 'custom' | 'escapism' | 'guro' | 'moot' | 'end' | 'aboba' | 'operative')}>
            <option value="default">{t('settings.presetDefault')}</option>
            <option value="escapism">{t('settings.presetEscapism')}</option>
            <option value="guro">{t('settings.presetGuro')}</option>
            <option value="moot">{t('settings.presetMoot')}</option>
            <option value="end">{t('settings.presetEnd')}</option>
            <option value="aboba">{t('settings.presetAboba')}</option>
            <option value="operative">{t('settings.presetOperative')}</option>
            <option value="custom">{t('settings.presetCustom')}</option>
          </Field>
        </div>
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
      </Fold>
      <Show when={hasStyleOptions()}>
        <Fold title={`${t('settings.groupStyleOptions')}: ${styleName()}`} defaultOpen followExpandFolds={false}>
          <Show when={uiStyle() === 'potato'}>
            <ToggleRow
              class="max-[899px]:hidden"
              label={t('settings.centerPotatoThreads')}
              checked={centerPotatoThreads()}
              onToggle={() => setCenterPotatoThreads(!centerPotatoThreads())}
            />
            <ToggleRow
              class="max-[899px]:hidden"
              label={t('settings.centerPotatoBoards')}
              checked={centerPotatoBoards()}
              onToggle={() => setCenterPotatoBoards(!centerPotatoBoards())}
            />
            <Show when={centerPotatoBoards()}>
              <div class={`${settingRowVariants()} max-[899px]:hidden`}>
                <span>{t('settings.potatoBoardsBar')}</span>
                <Field as="select" bordered select value={potatoBoardsBar()} onInput={(e) => setPotatoBoardsBar(e.currentTarget.value as PotatoBoardsBar)}>
                  <option value="pinned">{t('settings.potatoBoardsBarPinned')}</option>
                  <option value="scroll">{t('settings.potatoBoardsBarScroll')}</option>
                </Field>
              </div>
            </Show>
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
            <ToggleRow
              label={t('settings.potatoAsciiButtons')}
              checked={potatoAsciiButtons()}
              onToggle={() => setPotatoAsciiButtons(!potatoAsciiButtons())}
            />
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
            <ToggleRow label={t('settings.vistaBubbles')} checked={vistaBubbles()} onToggle={() => setVistaBubbles(!vistaBubbles())} />
          </Show>
        </Fold>
      </Show>
      <Fold title={t('settings.groupReactions')} defaultOpen followExpandFolds={false}>
        <ToggleRow label={t('settings.hideReactions')} checked={hideReactions()} onToggle={() => setHideReactions(!hideReactions())} />
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
      </Fold>
      <Fold title={t('settings.groupPosts')} defaultOpen followExpandFolds={false}>
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
        <ToggleRow label={t('settings.threadSeparator')} checked={threadSeparator()} onToggle={() => setThreadSeparator(!threadSeparator())} />
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
      </Fold>
      <Show when={hasAdvanced()}>
        <Fold title={t('settings.groupAdvanced')} followExpandFolds={false}>
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
          <ToggleRow label={t('settings.opPostBorders')} checked={opPostBorders()} onToggle={() => setOpPostBorders(!opPostBorders())} />
          <ToggleRow label={t('settings.opPostBackground')} checked={opPostBackground()} onToggle={() => setOpPostBackground(!opPostBackground())} />
        </Fold>
      </Show>
    </div>
  )
}
