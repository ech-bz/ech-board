import { createSignal } from 'solid-js'

const [open, setOpen] = createSignal(false)

export function mobileComposerOpen(): boolean {
  return open()
}

export function openMobileComposer(): void {
  setOpen(true)
}

export function closeMobileComposer(): void {
  setOpen(false)
}

export function toggleMobileComposer(): void {
  setOpen((v) => !v)
}
