import { createEffect, createSignal, onCleanup } from 'solid-js'

export function useIsMobile(): () => boolean {
  const [mobile, setMobile] = createSignal(window.matchMedia('(max-width: 899px)').matches)
  createEffect(() => {
    const query = window.matchMedia('(max-width: 899px)')
    const onChange = () => setMobile(query.matches)
    query.addEventListener('change', onChange)
    onCleanup(() => query.removeEventListener('change', onChange))
  })
  return mobile
}
