import { createSignal, Show, type JSX } from 'solid-js'
import { ApiCtx, type GalleryApi, type GalleryItem, type SourceRect } from './context'
import { GalleryOverlay } from './GalleryOverlay'

interface OpenState {
  items: GalleryItem[]
  index: number
  sourceRect: SourceRect | null
}

export function GalleryProvider(props: { children: JSX.Element }) {
  const [state, setState] = createSignal<OpenState | null>(null)
  const api: GalleryApi = {
    open: (items, index, sourceRect) => setState({ items, index, sourceRect: sourceRect ?? null }),
    close: () => setState(null),
  }
  return (
    <ApiCtx.Provider value={api}>
      {props.children}
      <Show when={state()}>
        {(open) => (
          <GalleryOverlay
            items={open().items}
            index={open().index}
            sourceRect={open().sourceRect}
            onClose={() => setState(null)}
          />
        )}
      </Show>
    </ApiCtx.Provider>
  )
}
