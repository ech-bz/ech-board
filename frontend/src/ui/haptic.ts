import { hapticTrigger } from 'ios-haptics'

function isDisabled(el: HTMLElement): boolean {
  return (el as HTMLButtonElement).disabled === true || el.getAttribute('aria-disabled') === 'true'
}

export function haptic(el: HTMLElement | null): (() => void) | undefined {
  if (!el) return undefined
  hapticTrigger(el)
  const overlay = el.lastElementChild as HTMLElement | null
  const sync = () => {
    if (!overlay) return
    overlay.style.display = isDisabled(el) ? 'none' : ''
  }
  sync()
  const observer = new MutationObserver(sync)
  observer.observe(el, { attributes: true, attributeFilter: ['disabled', 'aria-disabled'] })
  el.addEventListener('click', () => {
    if (isDisabled(el)) return
    navigator.vibrate?.(30)
  })
  return () => observer.disconnect()
}
