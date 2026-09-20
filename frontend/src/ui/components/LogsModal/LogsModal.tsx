import { Show } from 'solid-js'
import { Modal } from '../Modal'
import { EntityDebugBody, type EntityCodec, type EntityObject } from '../debug/EntityDebugBody'
import { ForumTab } from './ForumTab'
import { BoardTab } from './BoardTab'
import { useI18n } from '../../i18n'
import { ForumObject, BoardObject, ThreadObject, PostObject } from '../../../core/bcs/types'
import type { Ns } from '../../../core/events/core'

export type LogsTab = 'forum' | 'board' | 'thread' | 'post'

const TAB_NS: Record<LogsTab, Ns> = { forum: 'forum', board: 'board', thread: 'thread', post: 'post' }
const TAB_CODEC: Record<LogsTab, EntityCodec> = { forum: ForumObject, board: BoardObject, thread: ThreadObject, post: PostObject }

export function LogsModal(props: {
  tab: LogsTab
  relayUrl: string
  boardUid?: string
  object?: EntityObject | null
  onUpgrade?: (object: EntityObject) => void
  onClose: () => void
}) {
  const { t } = useI18n()
  return (
    <Modal title={t('logs.title')} onClose={props.onClose}>
      <Show
        when={props.object}
        fallback={
          <Show when={props.tab === 'forum'} fallback={<BoardTab relayUrl={props.relayUrl} boardUid={props.boardUid} />}>
            <ForumTab relayUrl={props.relayUrl} />
          </Show>
        }
      >
        {(object) => (
          <EntityDebugBody
            object={object()}
            codec={TAB_CODEC[props.tab]}
            ns={TAB_NS[props.tab]}
            relayUrl={props.relayUrl}
            onUpgrade={props.onUpgrade ? async (_ns, entity) => props.onUpgrade?.(entity) : undefined}
          />
        )}
      </Show>
    </Modal>
  )
}
