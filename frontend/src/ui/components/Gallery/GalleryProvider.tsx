import { createSignal, Show, type JSX } from 'solid-js'
import { ApiCtx, type GalleryApi, type GalleryItem, type SourceRect } from './context'
import { GalleryOverlay } from './GalleryOverlay'
import { MediaPlayer, isPlayableMime } from '../Player/MediaPlayer'
import { exitNativePip, onNativePipLeave } from '../Player/nativePip'

interface OpenState {
  items: GalleryItem[]
  index: number
  sourceRect: SourceRect | null
  resume: { index: number; media: HTMLMediaElement } | null
}

interface PipState {
  item: GalleryItem
  media: HTMLMediaElement
  items: GalleryItem[] | null
}

export function GalleryProvider(props: { children: JSX.Element }) {
  const [state, setState] = createSignal<OpenState | null>(null)
  const [pip, setPip] = createSignal<PipState | null>(null)
  const [hidden, setHidden] = createSignal(false)
  let stopNativeWatch: (() => void) | null = null
  const endNativePip = () => {
    stopNativeWatch?.()
    stopNativeWatch = null
    setHidden(false)
  }
  const api: GalleryApi = {
    open: (items, index, sourceRect) => {
      const item = items[index]
      if (item && isPlayableMime(item.mime)) setPip(null)
      endNativePip()
      setState({ items, index, sourceRect: sourceRect ?? null, resume: null })
    },
    close: () => {
      endNativePip()
      setState(null)
    },
  }
  const startPip = (item: GalleryItem, media: HTMLMediaElement) => {
    setPip({ item, media, items: state()?.items ?? null })
    setState(null)
  }
  const returnToGallery = (p: PipState) => {
    const index = p.items ? p.items.indexOf(p.item) : -1
    if (!p.items || index < 0) return
    endNativePip()
    setState({ items: p.items, index, sourceRect: null, resume: { index, media: p.media } })
    setPip(null)
  }
  const startNativePip = (media: HTMLMediaElement) => {
    stopNativeWatch?.()
    setHidden(true)
    stopNativeWatch = onNativePipLeave(media, () => {
      endNativePip()
      setState(null)
    })
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
            hidden={hidden()}
            resume={open().resume}
            onClose={() => {
              if (hidden()) exitNativePip()
              endNativePip()
              setState(null)
            }}
            onPip={startPip}
            onNativePip={startNativePip}
          />
        )}
      </Show>
      <Show when={pip()} keyed>
        {(p) => (
          <MediaPlayer
            floating
            item={p.item}
            adopt={p.media}
            onClose={() => setPip(null)}
            onToGallery={p.items ? () => returnToGallery(p) : undefined}
          />
        )}
      </Show>
    </ApiCtx.Provider>
  )
}
