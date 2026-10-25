import { useI18n } from '../../i18n'
import type { ModActionEntry } from '../../mod/types'

const pad = (n: number) => n.toString().padStart(2, '0')

export function useModFormat() {
  const { t, months } = useI18n()
  const date = (ts: number) => {
    const d = new Date(ts)
    return `${pad(d.getDate())} ${months()[d.getMonth()]} ${d.getFullYear()}`
  }
  const time = (ts: number) => {
    const d = new Date(ts)
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  }
  const place = (entry: Pick<ModActionEntry, 'place' | 'board' | 'threadNum'>) => {
    if (entry.place === 'forum' || !entry.board) return t('modUi.placeForum')
    if (entry.place === 'board' || entry.threadNum == null) return `/${entry.board}/`
    return `/${entry.board}/ · ${t('modUi.placeThread', { n: entry.threadNum })}`
  }
  return { date, time, full: (ts: number) => `${date(ts)} ${time(ts)}`, place }
}

export function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${Math.round(bytes / 1024)} KB`
}

export function matchesQuery(query: string, ...fields: (string | number | null | undefined)[]): boolean {
  const q = query.trim().toLowerCase().replace(/^[№#>]+/, '')
  if (!q) return true
  return fields.some((field) => field != null && String(field).toLowerCase().includes(q))
}
