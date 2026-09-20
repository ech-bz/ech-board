import { createEffect, createRoot, createSignal } from 'solid-js'
import type { Theme } from './i18n'

const [base, setBase] = createRoot(() => createSignal('ЭЧ'))
const [unread, setUnread] = createRoot(() => createSignal(0))
const [theme, setTheme] = createRoot(() => createSignal<Theme>('dark'))

createRoot(() => {
  createEffect(() => {
    const count = unread()
    document.title = count > 0 ? `(${count}) ${base()}` : base()
  })
  createEffect(() => {
    const dark = theme() === 'dark'
    const href = unread() > 0 ? (dark ? '/favicon_dark_unread.png' : '/favicon_light_unread.png') : dark ? '/favicon_dark.png' : '/favicon_light.png'
    let link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
    if (!link) {
      link = document.createElement('link')
      link.rel = 'icon'
      document.head.appendChild(link)
    }
    link.href = href
  })
})

export function setBaseTitle(value: string): void {
  setBase(value)
}

export function setUnreadTitle(count: number): void {
  setUnread(count)
}

export function setTitleTheme(value: Theme): void {
  setTheme(value)
}
