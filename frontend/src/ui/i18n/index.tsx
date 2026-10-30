import { createContext, createEffect, createSignal, useContext, type Accessor, type JSX } from 'solid-js'
import { ru } from './ru'
import type { Dict, MessageKey } from './ru'
import { en } from './en'
import { setTitleTheme } from '../title'

export type Lang = 'ru' | 'en'
export type Theme = 'light' | 'dark'
export type UiStyle = 'modern' | 'potato'

export const APPEARANCES: { id: string; label: string; file?: string; theme: Theme; enabled?: boolean }[] = [
  { id: 'default_dark', label: 'Default D', file: 'default_dark.css', theme: 'dark' },
  { id: 'default_light', label: 'Default L', file: 'default_light.css', theme: 'light' },
  { id: 'yotsuba', label: 'Yotsuba', file: 'yotsuba.css', theme: 'light' },
  { id: 'matrix', label: 'Matrix', file: 'matrix.css', theme: 'dark' },
  { id: 'gurochan', label: 'Gurochan', file: 'gurochan.css', theme: 'light' },
  { id: 'neutron', label: 'Neutron', file: 'neutron.css', theme: 'dark' },
  { id: 'jezowy', label: 'Jezowy', file: 'jezowy.css', theme: 'dark' },
  { id: 'dark_oled', label: 'Dark OLED', file: 'dark_oled.css', theme: 'dark' },
  { id: 'yotsuba_b', label: 'Yotsuba B', file: 'yotsuba_b.css', theme: 'light', enabled: false },
  { id: 'futaba', label: 'Futaba', file: 'futaba.css', theme: 'light', enabled: false },
  { id: 'burichan', label: 'Burichan', file: 'burichan.css', theme: 'light', enabled: false },
  { id: 'tomorrow', label: 'Tomorrow', file: 'tomorrow.css', theme: 'dark', enabled: false },
  { id: 'lord_english', label: 'Lord English', file: 'lord_english.css', theme: 'dark', enabled: false },
  { id: 'sosaka_light', label: 'Sosaka A', file: 'sosaka_a.css', theme: 'light' },
  { id: 'sosaka_dark', label: 'Sosaka B', file: 'sosaka_b.css', theme: 'dark' },
]

const dicts: Record<Lang, Dict> = { ru, en }

const APPEARANCE_CSS = import.meta.glob('../appearances/*.css', { query: '?url', import: 'default', eager: true }) as Record<string, string>

const LANG_KEY = 'ech-board-lang'
const THEME_KEY = 'ech-board-theme'
const APPEARANCE_KEY = 'ech-board-appearance'
const SHOW_DELETED_KEY = 'ech-board-show-deleted'
const HIDE_THREADS_KEY = 'ech-board-hide-threads'
const EXPAND_FOLDS_KEY = 'ech-board-expand-folds'
const HIDE_REACTIONS_KEY = 'ech-board-hide-reactions'
const POTATO_CENTER_THREADS_KEY = 'ech-board-potato-center-threads'
const POTATO_CENTER_BOARDS_KEY = 'ech-board-potato-center-boards'
const SHOW_GALLERY_ON_OPEN_KEY = 'ech-board-show-gallery-on-open'
const UI_STYLE_KEY = 'ech-board-ui-style'
const CORNER_RADIUS_KEY = 'ech-board-corner-radius'
const REACTION_SIZE_KEY = 'ech-board-reaction-size'
const CORNER_RADIUS_VERSION_KEY = 'ech-board-corner-radius-version'

function readString(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function writeString(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    return
  }
}

function initialAppearance(): string {
  const saved = readString(APPEARANCE_KEY)
  return saved && APPEARANCES.some((item) => item.id === saved) ? saved : 'default_dark'
}

function initialTheme(): Theme {
  const saved = readString(APPEARANCE_KEY)
  const found = APPEARANCES.find((item) => item.id === saved)
  if (found) return found.theme
  return readString(THEME_KEY) === 'light' ? 'light' : 'dark'
}

export interface I18nValue {
  uiStyle: Accessor<UiStyle>
  setUiStyle: (style: UiStyle) => void
  cornerRadius: Accessor<number>
  setCornerRadius: (value: number) => void
  reactionSize: Accessor<number>
  setReactionSize: (value: number) => void
  lang: Accessor<Lang>
  setLang: (l: Lang) => void
  theme: Accessor<Theme>
  setTheme: (t: Theme) => void
  appearance: Accessor<string>
  setAppearance: (id: string) => void
  showDeleted: Accessor<boolean>
  setShowDeleted: (v: boolean) => void
  hideThreadsCompletely: Accessor<boolean>
  setHideThreadsCompletely: (v: boolean) => void
  expandFolds: Accessor<boolean>
  setExpandFolds: (v: boolean) => void
  hideReactions: Accessor<boolean>
  setHideReactions: (v: boolean) => void
  centerPotatoThreads: Accessor<boolean>
  setCenterPotatoThreads: (v: boolean) => void
  centerPotatoBoards: Accessor<boolean>
  setCenterPotatoBoards: (v: boolean) => void
  showGalleryOnOpen: Accessor<boolean>
  setShowGalleryOnOpen: (v: boolean) => void
  t: (key: MessageKey, vars?: Record<string, string | number>) => string
  days: Accessor<readonly string[]>
  months: Accessor<readonly string[]>
}

const I18nContext = createContext<I18nValue>()

function boolPref(key: string, dflt: boolean): [Accessor<boolean>, (v: boolean) => void] {
  const stored = readString(key)
  const [value, setValue] = createSignal(stored === null ? dflt : stored === 'true')
  const set = (v: boolean) => {
    writeString(key, String(v))
    setValue(v)
  }
  return [value, set]
}

export function I18nProvider(props: { children: JSX.Element }) {
  const [lang, setLangSignal] = createSignal<Lang>(readString(LANG_KEY) === 'en' ? 'en' : 'ru')
  const [theme, setThemeSignal] = createSignal<Theme>(initialTheme())
  const [appearance, setAppearanceSignal] = createSignal<string>(initialAppearance())
  const [showDeleted, setShowDeleted] = boolPref(SHOW_DELETED_KEY, false)
  const [hideThreadsCompletely, setHideThreadsCompletely] = boolPref(HIDE_THREADS_KEY, false)
  const [expandFolds, setExpandFolds] = boolPref(EXPAND_FOLDS_KEY, false)
  const [hideReactions, setHideReactions] = boolPref(HIDE_REACTIONS_KEY, false)
  const [centerPotatoThreads, setCenterPotatoThreads] = boolPref(POTATO_CENTER_THREADS_KEY, false)
  const [centerPotatoBoards, setCenterPotatoBoards] = boolPref(POTATO_CENTER_BOARDS_KEY, false)
  const [showGalleryOnOpen, setShowGalleryOnOpen] = boolPref(SHOW_GALLERY_ON_OPEN_KEY, false)
  const storedStyle = readString(UI_STYLE_KEY)
  const [uiStyle, setUiStyleSignal] = createSignal<UiStyle>(storedStyle === 'potato' ? 'potato' : 'modern')
  const savedRadius = readString(CORNER_RADIUS_KEY)
  const radiusVersion = readString(CORNER_RADIUS_VERSION_KEY)
  const storedRadiusValue = savedRadius === null ? 100 : Number(savedRadius)
  const validStoredRadius = Number.isFinite(storedRadiusValue) ? Math.max(0, Math.min(100, storedRadiusValue)) : 100
  const migratedRadius = radiusVersion === '2' || savedRadius === null
    ? validStoredRadius
    : Math.max(0, Math.min(100, validStoredRadius * 2))
  const storedRadius = Math.round(migratedRadius / 10) * 10
  writeString(CORNER_RADIUS_VERSION_KEY, '2')
  if (savedRadius !== null && radiusVersion !== '2') writeString(CORNER_RADIUS_KEY, String(storedRadius))
  const [modernCornerRadius, setModernCornerRadiusSignal] = createSignal<number>(storedRadius)
  // Potato uses a fixed 10% corner scale; only Simple exposes the radius slider.
  const cornerRadius: Accessor<number> = () => uiStyle() === 'potato' ? 10 : modernCornerRadius()
  const savedReactionSize = readString(REACTION_SIZE_KEY)
  const reactionSizeValue = savedReactionSize === null ? 100 : Number(savedReactionSize)
  const [reactionSize, setReactionSizeSignal] = createSignal<number>(
    Math.round(Math.max(0, Math.min(100, Number.isFinite(reactionSizeValue) ? reactionSizeValue : 100)) / 10) * 10,
  )

  const setLang = (value: Lang) => {
    writeString(LANG_KEY, value)
    setLangSignal(value)
  }

  const setUiStyle = (style: UiStyle) => {
    writeString(UI_STYLE_KEY, style)
    setUiStyleSignal(style)
  }

  const setCornerRadius = (value: number) => {
    const next = Number.isFinite(value) ? Math.max(0, Math.min(100, Math.round(value / 10) * 10)) : 100
    if (uiStyle() === 'modern') {
      writeString(CORNER_RADIUS_KEY, String(next))
      setModernCornerRadiusSignal(next)
    }
  }

  const setReactionSize = (value: number) => {
    const next = Number.isFinite(value) ? Math.max(0, Math.min(100, Math.round(value / 10) * 10)) : 100
    writeString(REACTION_SIZE_KEY, String(next))
    setReactionSizeSignal(next)
  }

  const setTheme = (value: Theme) => {
    writeString(THEME_KEY, value)
    document.documentElement.setAttribute('data-theme', value)
    setTitleTheme(value)
    setThemeSignal(value)
  }

  const setAppearance = (id: string) => {
    const found = APPEARANCES.find((item) => item.id === id)
    if (!found) return
    writeString(APPEARANCE_KEY, id)
    document.documentElement.setAttribute('data-theme', found.theme)
    setTitleTheme(found.theme)
    setAppearanceSignal(id)
    setThemeSignal(found.theme)
  }

  createEffect(() => {
    document.documentElement.setAttribute('data-ui-style', uiStyle())
    const radius = cornerRadius()
    document.documentElement.setAttribute('data-corner-radius', String(radius))
    const cornerScale = radius / 100
    document.documentElement.style.setProperty('--corner-scale', String(cornerScale))
    document.documentElement.style.setProperty('--toggle-corner-scale', String(Math.sqrt(cornerScale)))
    document.documentElement.style.setProperty('--reaction-size', `${12 + reactionSize() * 0.12}px`)
    document.documentElement.style.setProperty('--reaction-glyph-size', `${12 + reactionSize() * (uiStyle() === 'potato' ? 0.12 : 0.04)}px`)
    document.documentElement.style.setProperty('--reaction-picker-size', `${12 + reactionSize() * 0.08}px`)
  })

  createEffect(() => {
    const found = APPEARANCES.find((item) => item.id === appearance())
    document.documentElement.setAttribute('data-appearance', found?.id ?? appearance())
    const href = found?.file ? APPEARANCE_CSS[`../appearances/${found.file}`] : null
    let link = document.querySelector<HTMLLinkElement>('link[data-appearance]')
    if (!href) {
      link?.remove()
      return
    }
    if (!link) {
      link = document.createElement('link')
      link.rel = 'stylesheet'
      link.setAttribute('data-appearance', '')
      document.head.appendChild(link)
    }
    link.href = href
  })

  createEffect(() => {
    const found = APPEARANCES.find((item) => item.id === appearance())
    if (!found) return
    document.documentElement.setAttribute('data-theme', found.theme)
    setTitleTheme(found.theme)
  })

  const t = (key: MessageKey, vars?: Record<string, string | number>) => {
    let s = dicts[lang()][key]
    if (vars) {
      for (const k of Object.keys(vars)) s = s.replace(`{${k}}`, String(vars[k]))
    }
    return s
  }

  const value: I18nValue = {
    uiStyle,
    setUiStyle,
    cornerRadius,
    setCornerRadius,
    reactionSize,
    setReactionSize,
    lang,
    setLang,
    theme,
    setTheme,
    appearance,
    setAppearance,
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
    t,
    days: () => dicts[lang()].days,
    months: () => dicts[lang()].months,
  }

  return <I18nContext.Provider value={value}>{props.children}</I18nContext.Provider>
}

export function useIsPotato(): Accessor<boolean> {
  const { uiStyle } = useI18n()
  return () => uiStyle() === 'potato'
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext)
  if (!value) throw new Error('useI18n must be used within I18nProvider')
  return value
}

