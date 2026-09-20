import { createResource, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { EntityDebugBody, type EntityObject } from '../debug/EntityDebugBody'
import { useI18n } from '../../i18n'
import { fetchBoardView } from '../../../core/api/relay'
import { BoardObject, BoardView as BoardViewCodec } from '../../../core/bcs/types'

const logStatusVariants = cva('my-1 text-md text-secondary')
const logErrorVariants = cva('my-1 text-md text-danger')

export function BoardTab(props: { relayUrl: string; boardUid?: string }) {
  const { t } = useI18n()
  const [board] = createResource(
    () => (props.boardUid ? { relayUrl: props.relayUrl, boardUid: props.boardUid } : null),
    (source) => fetchBoardView(source.relayUrl, source.boardUid).then((bytes) => BoardViewCodec.parse(new Uint8Array(bytes)).board),
  )
  return (
    <Show when={!board.error} fallback={<div class={logErrorVariants()}>{String(board.error)}</div>}>
      <Show when={board()} fallback={<p class={logStatusVariants()}>{props.boardUid ? t('common.loading') : t('logs.emptyBoard')}</p>}>
        {(value) => (
          <EntityDebugBody
            object={value() as unknown as EntityObject}
            codec={BoardObject}
            ns="board"
            relayUrl={props.relayUrl}
          />
        )}
      </Show>
    </Show>
  )
}
