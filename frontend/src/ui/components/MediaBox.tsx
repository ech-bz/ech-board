import { createSignal, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import type { SourceRect } from './Gallery/context'
import type { MediaInfo } from '../../core/media'
import { XIcon } from '../icons/XIcon'
import { MusicIcon } from '../icons/MusicIcon'
import { SearchIcon } from '../icons/SearchIcon'
import { DropdownMenu, type MenuItem } from './Menu/DropdownMenu'
import { useEntityActions } from '../entityActions'
import { fromHex } from '../../core/intent/crypto'
import type { PostRecord } from '../../store/records'

const mediaCardVariants = cva(
  'group relative flex-none h-media w-media cursor-pointer overflow-hidden rounded-md max-[899px]:h-media-mobile max-[899px]:w-media-mobile',
  {
    variants: {
      borderedUnbanned: {
        true: 'border border-dashed border-secondary',
      },
    },
  },
)

const mediaBannedVariants = cva('absolute inset-0 size-full object-cover')
const mediaThumbBlurVariants = cva('absolute inset-0 size-full scale-[1.15] object-cover blur-[28px]')
const mediaThumbVariants = cva('absolute inset-0 m-auto h-auto w-auto max-h-full max-w-full')
const mediaDurationVariants = cva('pointer-events-none absolute right-1.25 top-1.25 rounded-sm bg-shade px-1.5 py-0.5 text-xs leading-snug text-scrim-fg [font-variant-numeric:tabular-nums]')
const mediaModBtnVariants = cva('absolute left-0 top-0 z-control flex size-4 items-center justify-center bg-shade opacity-70 text-scrim-fg hover:opacity-100')
const mediaInfoVariants = cva('pointer-events-none absolute inset-x-0 bottom-0 flex flex-col gap-0.5 bg-shade px-2 py-1.5 text-xs leading-snug opacity-100 transition-opacity duration-150 group-hover:opacity-100 [@media(hover:hover)]:opacity-0 text-scrim-fg')
const mediaAudioThumbVariants = cva('absolute inset-0 flex items-center justify-center bg-shade text-scrim-fg')
const mediaPdfLabelVariants = cva('text-base font-bold tracking-wide')
const mediaSearchVariants = cva('absolute right-0 top-0 z-control flex')

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)}KB`
  return `${(bytes / 1048576).toFixed(1)}MB`
}

function formatDuration(ms: number): string {
  const total = Math.round(ms / 1000)
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

const MIME_LABELS: Record<string, string> = { 'audio/mpeg': 'MP3' }

function formatMime(mime: string): string {
  const i = mime.indexOf('/')
  const ext = i >= 0 ? mime.slice(i + 1) : mime
  return MIME_LABELS[mime] ?? ext.toUpperCase()
}

export function MediaBox(props: { m: MediaInfo; record: PostRecord; onOpen: (m: MediaInfo, rect?: SourceRect) => void }) {
  const { t } = useI18n()
  const actions = useEntityActions()
  const [natW, setNatW] = createSignal<number | null>(null)
  const isVideo = () => props.m.mime.startsWith('video/')
  const isAudio = () => props.m.mime.startsWith('audio/')
  const isDocument = () => props.m.mime === 'application/pdf'
  const isGif = () => props.m.mime === 'image/gif'
  const isImage = () => props.m.mime.startsWith('image/')
  const searchItems = (): MenuItem[] =>
    isImage() && !props.m.banned
      ? [
          { label: t('media.searchGoogle'), href: `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(props.m.fullUrl)}` },
          { label: t('media.searchYandex'), href: `https://yandex.ru/images/search?rpt=imageview&url=${encodeURIComponent(props.m.fullUrl)}` },
          { label: t('media.searchSaucenao'), href: `https://saucenao.com/search.php?db=999&url=${encodeURIComponent(props.m.fullUrl)}` },
          { label: t('media.searchIqdb'), href: `https://iqdb.org/?url=${encodeURIComponent(props.m.fullUrl)}` },
        ]
      : []
  const banMedia = () => actions.runBanMedia(props.record.post, props.record.threadUid, [fromHex(props.m.hash)])
  const unbanMedia = () => actions.runUnbanMedia(props.record.post, props.record.threadUid, [fromHex(props.m.hash)])
  return (
    <a
      data-media=""
      class={mediaCardVariants({ borderedUnbanned: (isVideo() || isGif() || isAudio() || isDocument()) && !props.m.banned })}
      href={props.m.banned ? undefined : props.m.fullUrl}
      target={isDocument() ? '_blank' : undefined}
      rel={isDocument() ? 'noopener noreferrer' : undefined}
      style={{ '--nat-w': natW() != null ? `${natW()}px` : undefined }}
      onClick={(e) => {
        if (props.m.banned) {
          e.preventDefault()
          return
        }
        if (isDocument()) return
        e.preventDefault()
        const rect = e.currentTarget.getBoundingClientRect()
        props.onOpen(props.m, { left: rect.left, top: rect.top, width: rect.width, height: rect.height })
      }}
    >
      <Show
        when={!props.m.banned}
        fallback={<img class={mediaBannedVariants()} src="/static/banned_media.png" alt="" />}
      >
        <Show
          when={!isAudio() && !isDocument()}
          fallback={
            <div class={mediaAudioThumbVariants()}>
              <Show when={isDocument()} fallback={<MusicIcon width={40} height={40} />}>
                <span class={mediaPdfLabelVariants()}>PDF</span>
              </Show>
            </div>
          }
        >
          <img class={mediaThumbBlurVariants()} src={props.m.thumbUrl} alt="" aria-hidden="true" />
          <img
            class={mediaThumbVariants()}
            src={props.m.thumbUrl}
            alt=""
            loading="lazy"
            decoding="async"
            onLoad={(e) => setNatW(e.currentTarget.naturalWidth)}
          />
        </Show>
      </Show>
      <Show when={!props.m.banned && (isVideo() || isAudio()) && props.m.durationMs != null}>
        <span class={mediaDurationVariants()}>{formatDuration(props.m.durationMs!)}</span>
      </Show>
      <Show when={!props.m.banned && searchItems().length > 0}>
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
      <Show when={props.record.canBanMedia && !props.m.banned}>
        <button
          type="button"
          class={mediaModBtnVariants()}
          title={t('mod.banMedia')}
          onClick={(e) => {
            e.preventDefault()
            e.stopPropagation()
            banMedia()
          }}
        >
          <XIcon width={12} height={12} />
        </button>
      </Show>
      <Show when={props.record.canUnbanMedia && props.m.banned}>
        <button
          type="button"
          class={mediaModBtnVariants()}
          title={t('mod.unbanMedia')}
          onClick={(e) => {
            e.preventDefault()
            e.stopPropagation()
            unbanMedia()
          }}
        >
          ↺
        </button>
      </Show>
      <Show when={!props.m.banned}>
        <div class={mediaInfoVariants()}>
          <span>
            {formatMime(props.m.mime)}
            {isAudio() || isDocument() ? '' : ` ${props.m.width}x${props.m.height}`}
          </span>
          <span>
            {formatSize(props.m.size)}
            {props.m.durationMs != null ? ` ${formatDuration(props.m.durationMs)}` : ''}
          </span>
        </div>
      </Show>
    </a>
  )
}
