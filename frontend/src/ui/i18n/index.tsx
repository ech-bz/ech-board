import { createContext, createEffect, createSignal, useContext, type Accessor, type JSX } from 'solid-js'
import { ru } from './ru'
import type { Dict, MessageKey } from './ru'
import { en } from './en'
import { setTitleTheme } from '../title'

export type Lang = 'ru' | 'en'
export type Theme = 'light' | 'dark'

export const APPEARANCES: { id: string; label: string; file?: string; theme: Theme; enabled?: boolean }[] = [
  { id: 'default_dark', label: 'Default D', file: 'default_dark.css', theme: 'dark' },
  { id: 'default_light', label: 'Default L', file: 'default_light.css', theme: 'light' },
  { id: 'yotsuba', label: 'Yotsuba', file: 'yotsuba.css', theme: 'light' },
  { id: 'matrix', label: 'Matrix', file: 'matrix.css', theme: 'dark' },
  { id: 'gurochan', label: 'Gurochan', file: 'gurochan.css', theme: 'light' },
  { id: 'neutron', label: 'Neutron', file: 'neutron.css', theme: 'dark' },
  { id: 'dark_oled', label: 'Dark OLED', file: 'dark_oled.css', theme: 'dark' },
  { id: 'yotsuba_b', label: 'Yotsuba B', file: 'yotsuba_b.css', theme: 'light', enabled: false },
  { id: 'futaba', label: 'Futaba', file: 'futaba.css', theme: 'light', enabled: false },
  { id: 'burichan', label: 'Burichan', file: 'burichan.css', theme: 'light', enabled: false },
  { id: 'tomorrow', label: 'Tomorrow', file: 'tomorrow.css', theme: 'dark', enabled: false },
  { id: 'lord_english', label: 'Lord English', file: 'lord_english.css', theme: 'dark', enabled: false },
  { id: 'sosaka_light', label: 'Sosaka Light', file: 'sosaka light.css', theme: 'light', enabled: false },
  { id: 'sosaka_dark', label: 'Sosaka Dark', file: 'sosaka dark.css', theme: 'dark', enabled: false },
]

const dicts: Record<Lang, Dict> = { ru, en }

const APPEARANCE_CSS = import.meta.glob('../appearances/*.css', { query: '?url', import: 'default', eager: true }) as Record<string, string>

const LANG_KEY = 'ech-board-lang'
const THEME_KEY = 'ech-board-theme'
const APPEARANCE_KEY = 'ech-board-appearance'
const SHOW_DELETED_KEY = 'ech-board-show-deleted'
const HIDE_THREADS_KEY = 'ech-board-hide-threads'

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

  const setLang = (value: Lang) => {
    writeString(LANG_KEY, value)
    setLangSignal(value)
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
    const found = APPEARANCES.find((item) => item.id === appearance())
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
    t,
    days: () => dicts[lang()].days,
    months: () => dicts[lang()].months,
  }

  return <I18nContext.Provider value={value}>{props.children}</I18nContext.Provider>
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext)
  if (!value) throw new Error('useI18n must be used within I18nProvider')
  return value
}
