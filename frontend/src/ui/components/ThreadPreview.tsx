import { createSignal, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n, useIsPotato, useIsVista } from '../i18n'
import { getHiddenThreads, setThreadHidden } from '../hiddenThreads'
import { Button } from './Button'
import { PostBody } from './Post'
import { useRecord, useStore } from '../store'
import { threadPath } from '../route'
import type { ThreadRowRecord } from '../../store/rows'

const sectionVariants = cva('', {
  variants: { potato: { true: 'border-t border-separator py-1 first:border-t-0 first:pt-0', false: 'mb-6 pr-3.5' } },
  defaultVariants: { potato: false },
})
const hiddenLabelVariants = cva('cursor-pointer px-0.5 py-1 text-lg italic text-secondary')
const separatorVariants = cva('mt-6 h-px bg-separator opacity-50')
const topicRowVariants = cva('flex items-center gap-2 px-1 pt-2')
const topicVariants = cva('min-w-0 flex-1 text-xl font-bold [overflow-wrap:anywhere]')
const postsColumnVariants = cva('flex w-fit max-w-full flex-col')
const potatoSkippedVariants = cva('flex items-center gap-1.5 pb-1 text-sm text-secondary')
const actionRowVariants = cva('flex items-center gap-2 pb-2 max-w-post')
const skippedVariants = cva('text-sm text-secondary')
const vistaSkippedVariants = cva('text-sm text-secondary mb-4')
const spacerVariants = cva('flex-1')

export function ThreadPreview(props: {
  row: ThreadRowRecord
  forumId: string
  slug?: string
  onOpen: () => void
  boardTitle?: string
}) {
  const { t, hideThreadsCompletely } = useI18n()
  const potato = useIsPotato()
  const vista = useIsVista()
  const store = useStore()
  const [hidden, setHidden] = createSignal(getHiddenThreads().has(props.row.uid))
  const op = useRecord(props.row.opUid ?? '')
  const ctx = () =>
    store.decryptCtx() && store.info()
      ? { decryptCtx: store.decryptCtx()!, relayUrl: store.info()!.relayUrl, forumId: store.info()!.forumId, boardUid: store.info()!.boardUid }
      : null
  const opView = () => {
    const record = op()
    const context = ctx()
    return record && context ? { record, ctx: context } : null
  }
  const hide = () => {
    setThreadHidden(props.row.uid, true)
    setHidden(true)
  }
  const openThread = (e: MouseEvent) => {
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
    e.preventDefault()
    props.onOpen()
  }
  const threadHref = () => (props.slug ? threadPath(props.forumId, props.slug, props.row.number) : undefined)
  const potatoActions = () => (
    <button type="button" class="potato-hide-thread" title={t('thread.hide')} aria-label={t('thread.hide')} onClick={hide}>✖</button>
  )
  return (
    <Show when={!hidden() || !hideThreadsCompletely()}>
      <Show
        when={!hidden()}
        fallback={
          <section class={sectionVariants({ potato: potato() })}>
            <div
              class={hiddenLabelVariants()}
              data-on-wallpaper=""
              onClick={() => {
                setThreadHidden(props.row.uid, false)
                setHidden(false)
              }}
            >
              <em>
                {t('thread.hidden', { num: props.row.number })}
                {props.row.title ? ` (${props.row.title})` : ''}
              </em>
            </div>
            <Show when={!potato()}>
              <div class={separatorVariants()} />
            </Show>
          </section>
        }
      >
        <section
          class={sectionVariants({ potato: potato() })}
          data-post-column=""
          data-potato-thread={potato() ? '' : undefined}
          data-vista-thread={vista() ? '' : undefined}
        >
          <Show when={props.row.topic && !vista()}>
            <div class={topicRowVariants()}>
              <div class={topicVariants()}>{props.row.topic}</div>
            </div>
          </Show>
          <div class={postsColumnVariants()} data-potato-posts-column={potato() ? '' : undefined}>
            <Show when={opView()}>
              {(view) => (
                <PostBody
                  record={view().record}
                  ctx={view().ctx}
                  threadClosed={props.row.closed}
                  threadBroom={props.row.opAdmin}
                  pinned={props.row.pinned}
                  slug={props.slug}
                  threadNum={props.row.number}
                  boardTitle={props.boardTitle}
                  headerActions={potato() ? potatoActions() : undefined}
                  onOpenThread={() => props.onOpen()}
                  title={vista() ? props.row.topic || undefined : undefined}
                  onHide={vista() ? hide : undefined}
                  boardPage
                />
              )}
            </Show>
            <Show
              when={!potato() && !vista()}
              fallback={
                <Show when={props.row.skipped > 0}>
                  <div class={vista() ? vistaSkippedVariants() : potatoSkippedVariants()} data-on-wallpaper={vista() ? '' : undefined}>
                    <Show when={potato()}>
                      <button type="button" class="potato-menu-trigger" title={t('thread.open')} aria-label={t('thread.open')} onClick={() => props.onOpen()}>⤢</button>
                    </Show>
                    {t('thread.skipped', { n: props.row.skipped })}
                  </div>
                </Show>
              }
            >
              <div class={actionRowVariants()}>
                <Show when={props.row.skipped > 0}>
                  <span class={skippedVariants()} data-on-wallpaper="">{t('thread.skipped', { n: props.row.skipped })}</span>
                </Show>
                <span class={spacerVariants()} />
                <Button size="sm" intent="accent" onClick={hide}>
                  {t('thread.hide')}
                </Button>
                <Button size="sm" intent="accent" href={threadHref()} onClick={openThread}>
                  {t('thread.open')}
                </Button>
              </div>
            </Show>
          </div>
          {props.row.replyUids.map((uid) => (
            <ThreadPreviewReply uid={uid} row={props.row} slug={props.slug} boardTitle={props.boardTitle} />
          ))}
          <Show when={!potato()}>
            <div class={separatorVariants()} />
          </Show>
        </section>
      </Show>
    </Show>
  )
}

function ThreadPreviewReply(props: { uid: string; row: ThreadRowRecord; slug?: string; boardTitle?: string }) {
  const store = useStore()
  const record = useRecord(props.uid)
  const view = () => {
    const rec = record()
    if (!rec) return null
    const info = store.info()
    const decryptCtx = store.decryptCtx()
    if (!info || !decryptCtx) return null
    return {
      record: rec,
      ctx: { decryptCtx, relayUrl: info.relayUrl, forumId: info.forumId, boardUid: info.boardUid },
    }
  }
  return (
    <Show when={view()}>
      {(entry) => (
        <PostBody
          record={entry().record}
          ctx={entry().ctx}
          threadClosed={props.row.closed}
          threadBroom={props.row.opAdmin}
          slug={props.slug}
          threadNum={props.row.number}
          boardTitle={props.boardTitle}
          boardPage
        />
      )}
    </Show>
  )
}
