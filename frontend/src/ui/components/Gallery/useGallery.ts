import { useContext } from 'solid-js'
import { ApiCtx, type GalleryItem, type SourceRect } from './context'
import { useStore } from '../../store'
import { useI18n } from '../../i18n'

export function useGallery() {
  const api = useContext(ApiCtx)
  const store = useStore()
  const { showDeleted } = useI18n()
  const openMedia = (item: GalleryItem, sourceRect?: SourceRect) => {
    const items = store.galleryOf(showDeleted())
    const index = items.findIndex((entry) => entry.fullUrl === item.fullUrl)
    if (index >= 0) api.open([...items], index, sourceRect)
    else api.open([item], 0, sourceRect)
  }
  return { openMedia }
}
