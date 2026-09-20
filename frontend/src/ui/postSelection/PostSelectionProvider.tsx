import { createContext, createEffect, createSignal, Show, useContext, type Accessor, type JSX } from 'solid-js'
import { parseMoveAbort } from '../../core/intent/errors'
import { reportError } from '../errors'
import { useEntityActions } from '../entityActions'
import { useI18n } from '../i18n'
import { useStore } from '../store'
import { ProgressPill } from './ProgressPill'
import type { PostObject } from '../../core/bcs/types'

export interface PostSelectionValue {
  canSelect: Accessor<boolean>
  selected: Accessor<Set<string>>
  hasSelection: Accessor<boolean>
  toggle: (id: string, shift: boolean) => void
  clear: () => void
  selectAllFromAuthor: (ref: PostObject, threadUid: string) => void
}

export const PostSelectionContext = createContext<PostSelectionValue | null>(null)

export function PostSelectionProvider(props: { canSelect: boolean; uids: Accessor<readonly string[]>; children: JSX.Element }) {
  const store = useStore()
  const ea = useEntityActions()
  const { t } = useI18n()
  const [selected, setSelected] = createSignal<Set<string>>(new Set())
  const [authorRun, setAuthorRun] = createSignal<{ done: number; total: number } | null>(null)
  let lastSelected: string | null = null

  const toggle = (id: string, shift: boolean) => {
    const ordered = props.uids()
    const previous = lastSelected
    setSelected((prev) => {
      const next = new Set(prev)
      if (shift && previous) {
        const b = ordered.indexOf(id)
        let a = ordered.indexOf(previous)
        if (a < 0 && prev.size > 0) {
          let best = -1
          let bestDistance = Number.POSITIVE_INFINITY
          for (let i = 0; i < ordered.length; i++) {
            if (prev.has(ordered[i])) {
              const distance = Math.abs(i - b)
              if (distance < bestDistance) {
                bestDistance = distance
                best = i
              }
            }
          }
          a = best
        }
        if (a >= 0 && b >= 0) {
          const lo = Math.min(a, b)
          const hi = Math.max(a, b)
          for (let i = lo; i <= hi; i++) next.add(ordered[i])
        } else {
          if (next.has(id)) next.delete(id)
          else next.add(id)
        }
      } else {
        if (next.has(id)) next.delete(id)
        else next.add(id)
      }
      return next
    })
    lastSelected = id
  }

  const clear = () => {
    setSelected(new Set<string>())
    lastSelected = null
  }

  createEffect(() => {
    const ids = new Set(props.uids())
    setSelected((prev) => {
      let changed = false
      const next = new Set<string>()
      for (const id of prev) {
        if (ids.has(id)) next.add(id)
        else changed = true
      }
      if (!changed) return prev
      if (next.size === 0) lastSelected = null
      return next
    })
  })

  const selectAllFromAuthor = (ref: PostObject, threadUid: string) => {
    if (authorRun()) return
    const posts: PostObject[] = []
    for (const uid of props.uids()) {
      const record = store.get(uid)
      if (record) posts.push(record.post)
    }
    setAuthorRun({ done: 0, total: posts.length })
    ea.findAuthorPosts(ref, posts, threadUid, (done, total) => setAuthorRun({ done, total }))
      .then((ids) => {
        setSelected((prev) => new Set([...prev, ...ids]))
        lastSelected = null
      })
      .catch((e) => reportError(parseMoveAbort(e)?.message ?? String(e)))
      .finally(() => setAuthorRun(null))
  }

  const value: PostSelectionValue = {
    canSelect: () => props.canSelect,
    selected,
    hasSelection: () => selected().size > 0,
    toggle,
    clear,
    selectAllFromAuthor,
  }
  return (
    <PostSelectionContext.Provider value={value}>
      <Show when={authorRun()}>
        {(progress) => <ProgressPill label={t('select.progress', { done: progress().done, total: progress().total })} />}
      </Show>
      {props.children}
    </PostSelectionContext.Provider>
  )
}

export function usePostSelection(): PostSelectionValue | null {
  return useContext(PostSelectionContext)
}
