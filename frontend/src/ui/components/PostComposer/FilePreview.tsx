import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { XIcon } from '../../icons/XIcon'
import { MusicIcon } from '../../icons/MusicIcon'
import { IconButton } from '../IconButton'

const filesRowVariants = cva('flex flex-wrap items-end gap-2')
const fileItemVariants = cva('relative rounded-md bg-panel2 p-1')
const fileMediaVariants = cva('block size-18 rounded-sm object-cover')
const fileAudioVariants = cva('flex size-18 items-center justify-center rounded-sm bg-shade text-fg')
const filePdfVariants = cva('flex size-18 items-center justify-center rounded-sm bg-shade text-xs font-bold tracking-wider text-fg')
const fileMetaVariants = cva('mt-0.5 flex items-center justify-between px-0.5 text-sm text-secondary')
const fileRemoveVariants = cva('p-0 leading-none hover:bg-hover')

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)}KB`
  return `${(bytes / 1048576).toFixed(1)}MB`
}

export function FilePreview(props: { files: File[]; previewUrls: Map<File, string>; onRemove: (idx: number) => void }) {
  return (
    <div class={filesRowVariants()}>
      <For each={props.files}>
        {(file, index) => (
          <div class={fileItemVariants()}>
            <Show
              when={file.type.startsWith('video/')}
              fallback={
                <Show
                  when={file.type.startsWith('audio/')}
                  fallback={
                    <Show
                      when={file.type === 'application/pdf'}
                      fallback={<img class={fileMediaVariants()} src={props.previewUrls.get(file)} alt={file.name} />}
                    >
                      <div class={filePdfVariants()}>PDF</div>
                    </Show>
                  }
                >
                  <div class={fileAudioVariants()}>
                    <MusicIcon width={22} height={22} />
                  </div>
                </Show>
              }
            >
              <video class={fileMediaVariants()} src={props.previewUrls.get(file)} muted playsinline preload="metadata" />
            </Show>
            <div class={fileMetaVariants()}>
              <span>{formatSize(file.size)}</span>
              <IconButton size="sm" class={fileRemoveVariants()} onClick={() => props.onRemove(index())}>
                <XIcon width={14} height={14} />
              </IconButton>
            </div>
          </div>
        )}
      </For>
    </div>
  )
}
