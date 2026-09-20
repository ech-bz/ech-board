import { createContext, createEffect, createMemo, createSignal, onCleanup, useContext, type Accessor, type JSX } from 'solid-js'
import type { MediaInfo } from '../core/media'
import type { PostRecord } from '../store/records'
import type { ThreadRowRecord } from '../store/rows'
import type { BoardPageInfo, ForumStore, StaticInfo, ThreadPageInfo } from '../store/store'

const StoreCtx = createContext<ForumStore | null>(null)

export function StoreProvider(props: { store: ForumStore; children: JSX.Element }) {
  return <StoreCtx.Provider value={props.store}>{props.children}</StoreCtx.Provider>
}

export function useStore(): ForumStore {
  const store = useContext(StoreCtx)
  if (!store) throw new Error('store is missing')
  return store
}

function useRevision(subscribe: (listener: () => void) => () => void): Accessor<number> {
  const [rev, setRev] = createSignal(0)
  onCleanup(subscribe(() => setRev((value) => value + 1)))
  return rev
}

export function useRecord(uid: string): Accessor<PostRecord | undefined> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribe(uid, listener))
  return createMemo(() => {
    rev()
    return store.get(uid)
  })
}

export function useRecordSource(uid: Accessor<string>): Accessor<PostRecord | undefined> {
  const store = useStore()
  const [rev, setRev] = createSignal(0)
  createEffect(() => {
    const id = uid()
    if (!id) return
    onCleanup(store.subscribe(id, () => setRev((value) => value + 1)))
  })
  return createMemo(() => {
    const id = uid()
    rev()
    return id ? store.get(id) : undefined
  })
}

export function usePage(): Accessor<ThreadPageInfo | null> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribePage(listener))
  return createMemo(() => {
    rev()
    return store.page()
  })
}

export function useBoardInfo(): Accessor<BoardPageInfo | null> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribePage(listener))
  return createMemo(() => {
    rev()
    return store.boardInfo()
  })
}

export function useLoadingMore(): Accessor<boolean> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribePage(listener))
  return createMemo(() => {
    rev()
    return store.loadingMoreThreads()
  })
}

export function useStaticInfo(): Accessor<StaticInfo | null> {
  const store = useStore()
  return createMemo(() => store.info())
}

export function usePosts(showDeleted: Accessor<boolean>): Accessor<readonly string[]> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribePosts(listener))
  return createMemo(() => {
    rev()
    return store.postsOf(showDeleted())
  })
}

export function useRows(showDeleted: Accessor<boolean>): Accessor<readonly ThreadRowRecord[]> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribeRows(listener))
  return createMemo(() => {
    rev()
    return store.rowsOf(showDeleted())
  })
}

export function useGallery(showDeleted: Accessor<boolean>): Accessor<readonly MediaInfo[]> {
  const store = useStore()
  const rev = useRevision((listener) => store.subscribeGallery(listener))
  return createMemo(() => {
    rev()
    return store.galleryOf(showDeleted())
  })
}
