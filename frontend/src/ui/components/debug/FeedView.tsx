import { createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { fetchFeed } from '../../../core/api/relay'
import { FeedView as FeedViewBcs } from '../../../core/bcs/types'
import { parseEvent, type Ns, type DecodedEvent } from '../../../core/events/core'
import { toHex } from '../../../core/intent/crypto'
import { useI18n } from '../../i18n'
import { JsonTree, formatValue } from './JsonTree'

const statusVariants = cva('my-1 text-md text-secondary')
const errorVariants = cva('my-1 text-md text-danger')
const itemVariants = cva('overflow-hidden rounded-md border border-separator')
const summaryVariants = cva('cursor-pointer select-text overflow-hidden text-ellipsis whitespace-nowrap px-2 py-1 text-sm leading-snug text-secondary')
const idxVariants = cva('mr-1 text-sm text-secondary')
const kindVariants = cva('mr-1 text-sm font-bold text-accent')
const fieldsVariants = cva('text-sm text-secondary break-all')
const bodyVariants = cva('border-t border-separator px-1.5 py-1.5 font-mono text-sm text-secondary')
const hexVariants = cva('break-all text-secondary')

interface FeedItem {
  idx: number
  ev: DecodedEvent | { $kind: 'raw'; raw: Uint8Array }
}

function isRaw(ev: FeedItem['ev']): ev is { $kind: 'raw'; raw: Uint8Array } {
  return ev.$kind === 'raw'
}

function withoutMeta(ev: DecodedEvent): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const key of Object.keys(ev)) {
    if (key === '$kind') continue
    out[key] = (ev as unknown as Record<string, unknown>)[key]
  }
  return out
}

function collapsedFields(ev: DecodedEvent): string {
  const parts: string[] = []
  for (const key of Object.keys(ev.payload)) {
    const v = (ev.payload as Record<string, unknown>)[key]
    if (typeof v === 'string') parts.push(`${key}=${v.length > 10 ? `${v.slice(0, 10)}…` : v}`)
    else if (typeof v === 'bigint') parts.push(`${key}=${v}`)
    else if (typeof v === 'number' || typeof v === 'boolean') parts.push(`${key}=${v}`)
    else if (Array.isArray(v)) parts.push(`${key}=[${v.length}]`)
    else if (v != null && typeof v === 'object') parts.push(`${key}={…}`)
  }
  return parts.join(' ')
}

export function FeedView(props: { feedId: string; counter: number; relayUrl: string; ns: Ns }) {
  const { t } = useI18n()
  const [items, setItems] = createSignal<FeedItem[]>([])
  const [nextCursor, setNextCursor] = createSignal<number | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [loadingMore, setLoadingMore] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [expanded, setExpanded] = createSignal<Set<number>>(new Set())
  let sentinel: HTMLDivElement | undefined

  const parsePage = (rawItems: Uint8Array[], endOffset: number): FeedItem[] =>
    rawItems.map((bytes, index) => {
      const idx = endOffset - 1 - index
      try {
        let pos = 32
        let dataLen = 0
        let shift = 0
        while (pos < bytes.length) {
          const byte = bytes[pos++]
          dataLen |= (byte & 0x7f) << shift
          if ((byte & 0x80) === 0) break
          shift += 7
        }
        return { idx, ev: parseEvent(bytes.slice(pos, pos + dataLen), props.ns) }
      } catch {
        return { idx, ev: { $kind: 'raw', raw: bytes } }
      }
    })

  const loadFirst = async () => {
    setLoading(true)
    setError(null)
    setItems([])
    try {
      const bytes = await fetchFeed(props.relayUrl, props.feedId, props.counter)
      const page = FeedViewBcs.parse(new Uint8Array(bytes))
      setItems(parsePage(page.items.map((item) => new Uint8Array(item)), props.counter + 1))
      setNextCursor(page.next_cursor != null ? Number(page.next_cursor) : null)
    } catch (e) {
      setError(String(e))
    }
    setLoading(false)
  }

  const loadMore = async () => {
    const cursor = nextCursor()
    if (cursor == null || loadingMore()) return
    setLoadingMore(true)
    try {
      const bytes = await fetchFeed(props.relayUrl, props.feedId, props.counter, cursor)
      const page = FeedViewBcs.parse(new Uint8Array(bytes))
      setItems([...items(), ...parsePage(page.items.map((item) => new Uint8Array(item)), cursor)])
      setNextCursor(page.next_cursor != null ? Number(page.next_cursor) : null)
    } catch (e) {
      setError(String(e))
    }
    setLoadingMore(false)
  }

  const toggle = (idx: number) => {
    const next = new Set(expanded())
    if (next.has(idx)) next.delete(idx)
    else next.add(idx)
    setExpanded(next)
  }

  onMount(() => {
    void loadFirst()
  })

  onMount(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) void loadMore()
    }, { threshold: 0 })
    if (sentinel) observer.observe(sentinel)
    onCleanup(() => observer.disconnect())
  })

  return (
    <div>
      <Show when={loading()}>
        <p class={statusVariants()}>{t('feed.loading')}</p>
      </Show>
      <Show when={error()}>{(message) => <div class={errorVariants()}>{message()}</div>}</Show>
      <Show when={!loading() && items().length === 0}>
        <p class={statusVariants()}>{t('feed.noEvents')}</p>
      </Show>
      <For each={items()}>
        {(item, index) => {
          const ev = item.ev
          const raw = isRaw(ev)
          const isExpanded = () => expanded().has(index())
          return (
            <div class={itemVariants()}>
              <div
                class={summaryVariants()}
                onClick={() => {
                  const selection = window.getSelection()
                  if (selection && selection.toString().length > 0) return
                  toggle(index())
                }}
              >
                <span class={idxVariants()}>#{item.idx}</span>
                <span class={kindVariants()}>{raw ? 'raw' : (ev as DecodedEvent).$kind}</span>
                <Show when={!isExpanded() && !raw}>
                  <span class={fieldsVariants()}>{collapsedFields(ev as DecodedEvent)}</span>
                </Show>
              </div>
              <Show when={isExpanded()}>
                <div class={bodyVariants()}>
                  <Show when={!raw} fallback={<span class={hexVariants()}>{toHex((ev as { raw: Uint8Array }).raw)}</span>}>
                    <JsonTree data={formatValue(withoutMeta(ev as DecodedEvent))} />
                  </Show>
                </div>
              </Show>
            </div>
          )
        }}
      </For>
      <div ref={sentinel} style={{ height: '1px' }} />
      <Show when={loadingMore()}>
        <p class={statusVariants()}>{t('feed.loadingMore')}</p>
      </Show>
    </div>
  )
}
