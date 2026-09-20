import { createEffect } from 'solid-js'
import { useLocation } from '@solidjs/router'
import { parseRoute } from './route'
import { getCachedDefaultForumId } from '../core/config'
import { setBaseTitle } from './title'
import { useI18n } from './i18n'
import { usePage, useStore } from './store'
import type { BoardInfo } from '../core/views'

export function TitleSync(props: { boards: BoardInfo[] }) {
  const { t } = useI18n()
  const store = useStore()
  const page = usePage()
  const location = useLocation()
  createEffect(() => {
    const route = parseRoute(location.pathname + location.hash, getCachedDefaultForumId())
    const forumName = t('ui.forumName')
    if (route.page === 'board') {
      const board = props.boards.find((item) => item.slug === route.slug)
      setBaseTitle(board ? `/${route.slug}/ - ${board.title}` : `/${route.slug}/`)
      return
    }
    if (route.page === 'thread') {
      const current = page()
      const title = current ? store.threadTitle() : ''
      setBaseTitle(`/${route.slug}/ - ${title || `#${route.threadNum}`}`)
      return
    }
    if (route.page === 'compose') {
      setBaseTitle(`/${route.slug}/ - ${t('ui.newThread')}`)
      return
    }
    if (route.page === 'settings') {
      setBaseTitle(`${forumName} - ${t('ui.settings')}`)
      return
    }
    setBaseTitle(forumName)
  })
  return null
}
