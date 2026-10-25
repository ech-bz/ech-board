import { createEffect, createMemo, createSignal, For, on, Show, useContext } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Field } from '../Field'
import { Button } from '../Button'
import { ModMediaThumb } from './ModMedia'
import { ApiCtx, type GalleryItem, type SourceRect } from '../Gallery/context'
import { useI18n } from '../../i18n'
import { useNav } from '../../nav'
import { modData } from '../../mod/modData'
import type { ModFile } from '../../mod/types'

const PAGE = 48

const toolsVariants = cva('flex flex-none flex-wrap items-center gap-2')
const countVariants = cva('ml-auto text-sm text-secondary')
const panelVariants = cva('min-h-0 flex-1 overflow-y-auto overscroll-contain rounded-lg bg-panel1 p-2.5')
const gridVariants = cva('grid grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-2 min-[900px]:grid-cols-[repeat(auto-fill,minmax(140px,1fr))]')
const emptyVariants = cva('p-8 text-center text-secondary')

type TypeFilter = '' | 'img' | 'gif' | 'vid'

const kindOf = (mime: string): TypeFilter => (mime === 'image/gif' ? 'gif' : mime.startsWith('video/') ? 'vid' : 'img')
const dayKey = (ts: number) => {
  const d = new Date(ts)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function toGalleryItem(file: ModFile, openPost: (file: ModFile) => void): GalleryItem {
  return {
    thumbUrl: file.thumbUrl,
    fullUrl: file.fullUrl,
    width: file.width,
    height: file.height,
    mime: file.mime.startsWith('video/') ? 'image/jpeg' : file.mime,
    size: file.size,
    durationMs: null,
    postUid: String(file.postNum),
    threadUid: String(file.threadNum),
    hash: file.id,
    banned: false,
    info: { posterUid: file.authorUid, postNum: file.postNum, board: file.board, ts: file.ts, onOpenPost: () => openPost(file) },
  }
}

export function GalleryPanel(props: { forumId: string; boards: string[]; canModerate: boolean }) {
  const { t } = useI18n()
  const nav = useNav()
  const gallery = useContext(ApiCtx)
  const [date, setDate] = createSignal('')
  const [thread, setThread] = createSignal('')
  const [board, setBoard] = createSignal('')
  const [type, setType] = createSignal<TypeFilter>('')
  const [shown, setShown] = createSignal(PAGE)
  let scroller: HTMLDivElement | undefined

  const live = createMemo(() => modData.files().filter((f) => !f.deleted))
  const filtered = createMemo(() => {
    const threadQ = thread().trim().replace(/^[№#]+/, '')
    return live().filter(
      (f) => (!date() || dayKey(f.ts) === date()) && (!threadQ || String(f.threadNum) === threadQ) && (!board() || f.board === board()) && (!type() || kindOf(f.mime) === type()),
    )
  })
  const visible = createMemo(() => filtered().slice(0, shown()))

  createEffect(
    on([date, thread, board, type], () => {
      setShown(PAGE)
      scroller?.scrollTo({ top: 0 })
    }, { defer: true }),
  )

  const openPost = (file: ModFile) =>
    nav.navigate({ page: 'thread', forumId: props.forumId, slug: file.board, threadNum: file.threadNum, postNum: file.postNum })
  const openFile = (index: number, rect: SourceRect) => gallery.open(visible().map((f) => toGalleryItem(f, openPost)), index, rect)

  return (
    <div class="flex min-h-0 flex-1 flex-col gap-2">
      <div class={toolsVariants()}>
        <Field type="date" aria-label={t('modUi.date')} title={t('modUi.date')} value={date()} onInput={(e) => setDate(e.currentTarget.value)} />
        <Field
          type="text"
          inputmode="numeric"
          class="w-32 min-w-0"
          placeholder={t('modUi.threadNum')}
          aria-label={t('modUi.threadNum')}
          value={thread()}
          onInput={(e) => setThread(e.currentTarget.value)}
        />
        <Field as="select" aria-label={t('ui.boards')} value={board()} onInput={(e) => setBoard(e.currentTarget.value)}>
          <option value="">{t('modUi.allBoards')}</option>
          <For each={props.boards}>{(slug) => <option value={slug}>/{slug}/</option>}</For>
        </Field>
        <Field as="select" aria-label={t('modUi.typeAll')} value={type()} onInput={(e) => setType(e.currentTarget.value as TypeFilter)}>
          <option value="">{t('modUi.typeAll')}</option>
          <option value="img">{t('modUi.typeImages')}</option>
          <option value="gif">{t('modUi.typeGif')}</option>
          <option value="vid">{t('modUi.typeVideo')}</option>
        </Field>
        <span class={countVariants()}>{t('modUi.filesShown', { shown: visible().length, total: filtered().length })}</span>
      </div>
      <div ref={scroller} class={panelVariants()}>
        <Show when={filtered().length > 0} fallback={<div class={emptyVariants()}>{t('modUi.empty')}</div>}>
          <div class={gridVariants()}>
            <For each={visible()}>
              {(file, index) => (
                <ModMediaThumb
                  fill
                  thumbUrl={file.thumbUrl}
                  fullUrl={file.fullUrl}
                  mime={file.mime}
                  width={file.width}
                  height={file.height}
                  size={file.size}
                  onDelete={props.canModerate ? () => modData.deleteFile(file.id) : undefined}
                  deleteLabel={t('modUi.deleteFile')}
                  onOpen={(rect) => openFile(index(), rect)}
                />
              )}
            </For>
          </div>
          <Show when={visible().length < filtered().length}>
            <div class="flex justify-center pb-1 pt-3">
              <Button onClick={() => setShown((n) => n + PAGE)}>{t('modUi.more')}</Button>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  )
}
