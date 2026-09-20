import { createResource, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { EntityDebugBody, type EntityObject } from '../debug/EntityDebugBody'
import { useI18n } from '../../i18n'
import { fetchForum } from '../../../core/api/relay'
import { ForumObject } from '../../../core/bcs/types'

const logStatusVariants = cva('my-1 text-md text-secondary')
const logErrorVariants = cva('my-1 text-md text-danger')

export function ForumTab(props: { relayUrl: string; onUpgrade?: (object: EntityObject) => Promise<void> }) {
  const { t } = useI18n()
  const [forum] = createResource(
    () => props.relayUrl,
    (relayUrl) => fetchForum(relayUrl).then((view) => view.forum),
  )
  return (
    <Show when={!forum.error} fallback={<div class={logErrorVariants()}>{String(forum.error)}</div>}>
      <Show when={forum()} fallback={<p class={logStatusVariants()}>{t('common.loading')}</p>}>
        {(value) => (
          <EntityDebugBody
            object={value() as unknown as EntityObject}
            codec={ForumObject}
            ns="forum"
            relayUrl={props.relayUrl}
            onUpgrade={props.onUpgrade ? async (_ns, entity) => props.onUpgrade!(entity) : undefined}
          />
        )}
      </Show>
    </Show>
  )
}
