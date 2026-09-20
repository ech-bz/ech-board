const KEY = 'ech-board-hidden-threads'

export function getHiddenThreads(): Set<string> {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw) return new Set(JSON.parse(raw))
  } catch {}
  return new Set()
}

export function setThreadHidden(id: string, hidden: boolean): Set<string> {
  const s = getHiddenThreads()
  if (hidden) s.add(id)
  else s.delete(id)
  try { localStorage.setItem(KEY, JSON.stringify([...s])) } catch {}
  return s
}
