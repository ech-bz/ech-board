import { useNav } from './nav'

export function useMobileTabs() {
  const nav = useNav()
  const goHome = () => nav.section('boards')
  const openSettings = () => nav.section('settings')
  return { goHome, openSettings }
}
