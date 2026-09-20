import { createSignal, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import { getHiddenThreads, setThreadHidden } from '../hiddenThreads'
import { Button } from './Button'
import { PostBody } from './Post'
import { useRecord, useStore } from '../store'
import { threadPath } from '../route'
import type { ThreadRowRecord } from '../../store/rows'

const sectionVariants = cva('mb-6')
const hiddenLabelVariants = cva('cursor-pointer px-0.5 py-1 text-lg italic text-secondary')
const separatorVariants = cva('mt-6 h-px bg-separator opacity-50')
const topicRowVariants = cva('flex items-center gap-2 px-0.5 pb-2')
const topicVariants = cva('min-w-0 flex-1 text-xl font-bold [overflow-wrap:anywhere]')
const postsColumnVariants = cva('flex w-fit max-w-full flex-col')
const actionRowVariants = cva('flex items-center gap-2 pb-2 max-w-post')
const skippedVariants = cva('text-sm text-secondary')
const spacerVariants = cva('flex-1')

export function ThreadPreview(props: {
  row: ThreadRowRecord
  forumId: string
  slug?: string
  onOpen: () => void
}) {
  const { t, hideThreadsCompletely } = useI18n()
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
  return (
    <Show when={!hidden() || !hideThreadsCompletely()}>
      <Show
        when={!hidden()}
        fallback={
          <section class={sectionVariants()}>
            <div
              class={hiddenLabelVariants()}
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
            <div class={separatorVariants()} />
          </section>
        }
      >
        <section class={sectionVariants()}>
          <Show when={props.row.topic}>
            <div class={topicRowVariants()}>
              <div class={topicVariants()}>{props.row.topic}</div>
            </div>
          </Show>
          <div class={postsColumnVariants()}>
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
                  boardPage
                />
              )}
            </Show>
            <div class={actionRowVariants()}>
              <Show when={props.row.skipped > 0}>
                <span class={skippedVariants()}>{t('thread.skipped', { n: props.row.skipped })}</span>
              </Show>
              <span class={spacerVariants()} />
              <Button size="sm" intent="accent" onClick={hide}>
                {t('thread.hide')}
              </Button>
              <Button
                size="sm"
                intent="accent"
                href={props.slug ? threadPath(props.forumId, props.slug, props.row.number) : undefined}
                onClick={(e) => {
                  if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey) return
                  e.preventDefault()
                  props.onOpen()
                }}
              >
                {t('thread.open')}
              </Button>
            </div>
          </div>
          {props.row.replyUids.map((uid) => (
            <ThreadPreviewReply uid={uid} row={props.row} slug={props.slug} />
          ))}
          <div class={separatorVariants()} />
        </section>
      </Show>
    </Show>
  )
}

function ThreadPreviewReply(props: { uid: string; row: ThreadRowRecord; slug?: string }) {
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
          boardPage
        />
      )}
    </Show>
  )
}
