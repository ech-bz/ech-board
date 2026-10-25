import { createSignal, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { DropdownMenu, type MenuItem } from '../Menu/DropdownMenu'
import {
  formatDuration,
  formatMime,
  formatSize,
  mediaBannedVariants,
  mediaCardVariants,
  mediaDurationVariants,
  mediaInfoVariants,
  mediaModBtnVariants,
  mediaSearchVariants,
  mediaThumbBlurVariants,
  mediaThumbVariants,
} from '../MediaBox'
import { XIcon } from '../../icons/XIcon'
import { SearchIcon } from '../../icons/SearchIcon'
import { useI18n } from '../../i18n'
import type { SourceRect } from '../Gallery/context'

const tileVariants = cva('group relative aspect-square cursor-pointer overflow-hidden rounded-md bg-panel2')
const dashedVariants = cva('border border-dashed border-secondary')

export interface ModMediaThumbProps {
  thumbUrl: string
  fullUrl: string
  mime: string
  width: number
  height: number
  size: number
  durationMs?: number | null
  hidden?: boolean
  fill?: boolean
  onDelete?: () => void
  deleteLabel?: string
  onOpen: (rect: SourceRect) => void
}

export function ModMediaThumb(props: ModMediaThumbProps) {
  const { t } = useI18n()
  const [natW, setNatW] = createSignal<number | null>(null)
  const isVideo = () => props.mime.startsWith('video/')
  const isGif = () => props.mime === 'image/gif'
  const isImage = () => props.mime.startsWith('image/')
  const searchItems = (): MenuItem[] =>
    isImage() && !props.hidden
      ? [
          { label: t('media.searchGoogle'), href: `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(props.fullUrl)}` },
          { label: t('media.searchYandex'), href: `https://yandex.ru/images/search?rpt=imageview&url=${encodeURIComponent(props.fullUrl)}` },
          { label: t('media.searchSaucenao'), href: `https://saucenao.com/search.php?db=999&url=${encodeURIComponent(props.fullUrl)}` },
          { label: t('media.searchIqdb'), href: `https://iqdb.org/?url=${encodeURIComponent(props.fullUrl)}` },
        ]
      : []
  const dashed = () => (isVideo() || isGif()) && !props.hidden
  return (
    <a
      data-media=""
      draggable={false}
      class={`${props.fill ? tileVariants() : mediaCardVariants()} ${dashed() ? dashedVariants() : ''}`}
      href={props.hidden ? undefined : props.fullUrl}
      style={props.fill ? undefined : { '--nat-w': natW() != null ? `${natW()}px` : undefined }}
      onClick={(e) => {
        e.preventDefault()
        if (props.hidden) return
        const r = e.currentTarget.getBoundingClientRect()
        props.onOpen({ left: r.left, top: r.top, width: r.width, height: r.height })
      }}
    >
      <Show when={!props.hidden} fallback={<img class={mediaBannedVariants()} src="/static/banned_media.png" alt="" />}>
        <img class={mediaThumbBlurVariants()} src={props.thumbUrl} alt="" aria-hidden="true" draggable={false} />
        <img
          class={mediaThumbVariants()}
          src={props.thumbUrl}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          onLoad={(e) => setNatW(e.currentTarget.naturalWidth)}
        />
      </Show>
      <Show when={!props.hidden && isVideo() && props.durationMs != null}>
        <span class={mediaDurationVariants()}>{formatDuration(props.durationMs!)}</span>
      </Show>
      <Show when={!props.hidden && searchItems().length > 0}>
        <div class={mediaSearchVariants()}>
          <DropdownMenu
            icon={<SearchIcon width={12} height={12} />}
            ariaLabel={t('media.searchImage')}
            triggerSize="xs"
            triggerShape="square"
            class="bg-shade opacity-70 hover:opacity-100 [&_svg]:text-scrim-fg"
            items={searchItems()}
          />
        </div>
      </Show>
      <Show when={props.onDelete && !props.hidden}>
        <button
          type="button"
          class={mediaModBtnVariants()}
          title={props.deleteLabel}
          aria-label={props.deleteLabel}
          onClick={(e) => {
            e.preventDefault()
            e.stopPropagation()
            props.onDelete!()
          }}
        >
          <XIcon width={12} height={12} />
        </button>
      </Show>
      <Show when={!props.hidden}>
        <div class={mediaInfoVariants()}>
          <span>
            {formatMime(props.mime)} {props.width}x{props.height}
          </span>
          <span>{formatSize(props.size)}</span>
        </div>
      </Show>
    </a>
  )
}
