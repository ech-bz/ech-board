import { createContext, useContext, type JSX } from 'solid-js'
import { useLocation, useNavigate } from '@solidjs/router'
import { getCachedDefaultForumId } from '../core/config'
import { depthOf, parentRoute, parseRoute, routeToPath, type Route } from './route'
export type Section = 'boards' | 'settings'

export interface Nav {
  navigate: (route: Route) => void
  navigatePath: (path: string) => void
  back: () => void
  section: (section: Section) => void
}

const NavContext = createContext<Nav | null>(null)

let lastContentPath = '/'
let currentForumId = ''

export function rememberContentPath(path: string, forumId: string): void {
  lastContentPath = path
  currentForumId = forumId
}

export function NavProvider(props: { children: JSX.Element }) {
  const router = useNavigate()
  const location = useLocation()
  let canGoBack = false
  const current = () => parseRoute(location.pathname + location.hash, getCachedDefaultForumId())
  const go = (route: Route) => {
    const from = current()
    const to = routeToPath(route)
    if (routeToPath(from) === to) return
    const fromSettings = from.page === 'settings'
    const toSettings = route.page === 'settings'
    const replace = fromSettings ? true : toSettings ? false : depthOf(route) <= depthOf(from)
    if (!replace) canGoBack = true
    router(to, { replace })
  }
  const navigatePath = (path: string) => go(parseRoute(path, getCachedDefaultForumId()))
  const back = () => {
    if (canGoBack) {
      router(-1)
      return
    }
    const parent = parentRoute(current())
    if (parent) go(parent)
  }
  const section = (target: Section) => {
    const from = current()
    if (target === 'settings') {
      go({ page: 'settings' })
      return
    }
    if (from.page === 'settings') {
      if (canGoBack) {
        router(-1)
        return
      }
      navigatePath(lastContentPath)
      return
    }
    if (from.page === 'boards') return
    go({ page: 'boards', forumId: currentForumId })
  }
  return <NavContext.Provider value={{ navigate: go, navigatePath, back, section }}>{props.children}</NavContext.Provider>
}

export function useNav(): Nav {
  const nav = useContext(NavContext)
  if (!nav) throw new Error('nav is missing')
  return nav
}
