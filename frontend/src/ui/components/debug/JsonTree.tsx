import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { toHex } from '../../../core/intent/crypto'

const valueVariants = cva('break-all text-secondary')
const contentsVariants = cva('contents')
const rowVariants = cva('whitespace-pre-wrap break-all')
const indexVariants = cva('text-secondary')
const keyVariants = cva('text-accent')

export function formatValue(v: unknown): unknown {
  if (v instanceof Uint8Array) return toHex(v)
  if (Array.isArray(v) && v.length > 0 && v.every((x) => typeof x === 'number' && Number.isInteger(x) && x >= 0 && x <= 255)) {
    return toHex(new Uint8Array(v as number[]))
  }
  if (typeof v === 'bigint') return v.toString()
  if (Array.isArray(v)) return v.map(formatValue)
  if (v != null && typeof v === 'object') {
    const out: Record<string, unknown> = {}
    for (const k of Object.keys(v)) out[k] = formatValue((v as Record<string, unknown>)[k])
    return out
  }
  return v
}

export function JsonTree(props: { data: unknown; depth?: number }) {
  const depth = () => props.depth ?? 0
  const keys = () => (props.data != null && typeof props.data === 'object' && !Array.isArray(props.data) ? Object.keys(props.data as object) : [])
  return (
    <Show
      when={props.data != null && typeof props.data === 'object'}
      fallback={<span class={valueVariants()}>{props.data == null ? 'null' : String(props.data)}</span>}
    >
      <Show
        when={Array.isArray(props.data)}
        fallback={
          <Show when={keys().length > 0} fallback={<span class={valueVariants()}>{'{ }'}</span>}>
            <span class={contentsVariants()}>
              <For each={keys()}>
                {(key) => (
                  <div class={rowVariants()} style={{ 'padding-left': `${depth() * 4}px` }}>
                    <span class={keyVariants()}>{key}:</span>{' '}
                    <JsonTree data={(props.data as Record<string, unknown>)[key]} depth={depth() + 1} />
                  </div>
                )}
              </For>
            </span>
          </Show>
        }
      >
        <Show when={(props.data as unknown[]).length > 0} fallback={<span class={valueVariants()}>[]</span>}>
          <span class={contentsVariants()}>
            <For each={props.data as unknown[]}>
              {(item, index) => (
                <div class={rowVariants()} style={{ 'padding-left': `${depth() * 4}px` }}>
                  <span class={indexVariants()}>[{index()}]</span> <JsonTree data={item} depth={depth() + 1} />
                </div>
              )}
            </For>
          </span>
        </Show>
      </Show>
    </Show>
  )
}
