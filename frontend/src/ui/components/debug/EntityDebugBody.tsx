import { Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { FeedView } from './FeedView'
import { JsonTree, formatValue } from './JsonTree'
import { toHex } from '../../../core/intent/crypto'
import { entityNs, CURRENT_ENTITY_VERSION, type EntityNs } from '../../../core/intent/upgrade'
import { useI18n } from '../../i18n'
import { Button } from '../Button'
import type { ForumObject, BoardObject, ThreadObject, PostObject } from '../../../core/bcs/types'
import type { Ns } from '../../../core/events/core'

const sectionVariants = cva('mb-4')
const hexFieldVariants = cva('w-full overflow-x-auto rounded-md bg-panel2 px-2.5 py-2 font-mono text-sm text-secondary')
const spacerVariants = cva('h-3')
const treeVariants = cva('max-h-debug overflow-y-auto rounded-md bg-panel2 p-2.5 font-mono text-sm leading-normal text-secondary')
const labelVariants = cva('mb-2 font-bold')
const feedColVariants = cva('flex max-h-debug flex-col gap-1 overflow-y-auto')

export type EntityObject = ForumObject | BoardObject | ThreadObject | PostObject
export type EntityCodec = typeof ForumObject | typeof BoardObject | typeof ThreadObject | typeof PostObject

export function EntityDebugBody(props: {
  object: EntityObject
  codec: EntityCodec
  ns: Ns
  relayUrl: string
  onUpgrade?: (ns: EntityNs, object: EntityObject) => Promise<void>
}) {
  const { t } = useI18n()
  const nsName = () => entityNs(props.codec)
  const needsUpgrade = () => !!props.onUpgrade && props.object.root.entity.version < CURRENT_ENTITY_VERSION[nsName()]
  const rawHex = () => {
    try {
      return toHex(new Uint8Array((props.codec as unknown as { serialize: (o: EntityObject) => { toBytes(): Uint8Array } }).serialize(props.object).toBytes()))
    } catch {
      return null
    }
  }
  return (
    <>
      <Show when={needsUpgrade()}>
        <div class={sectionVariants()}>
          <Button onClick={() => void props.onUpgrade!(nsName(), props.object)}>{t('mod.upgrade')}</Button>
        </div>
      </Show>
      <div class={sectionVariants()}>
        <Show when={rawHex()}>
          {(hex) => (
            <>
              <input class={hexFieldVariants()} readOnly value={hex().slice(2)} onFocus={(e) => e.currentTarget.select()} />
              <div class={spacerVariants()} />
            </>
          )}
        </Show>
        <div class={treeVariants()}>
          <JsonTree data={formatValue(props.object)} />
        </div>
      </div>
      <div class={sectionVariants()}>
        <div class={labelVariants()}>{t('feed.eventLog')}</div>
        <div class={feedColVariants()}>
          <FeedView
            feedId={toHex(props.object.root.entity.feed.id)}
            counter={Number(props.object.root.entity.feed.counter)}
            relayUrl={props.relayUrl}
            ns={props.ns}
          />
        </div>
      </div>
    </>
  )
}
