import { createSignal, Show } from 'solid-js'
import { useI18n } from '../i18n'
import { useEntityActions } from '../entityActions'
import { usePostSelection } from './PostSelectionProvider'
import { DropdownMenu } from '../components/Menu/DropdownMenu'
import { ProgressPill } from './ProgressPill'
import type { PostRecord } from '../../store/records'

/** A per-post counterpart of the moderator actions shown by ThreadSelectionMenu. */
export function SelectedPostMenu(props: { record: PostRecord; potato?: boolean }) {
  const { t } = useI18n()
  const ea = useEntityActions()
  const selection = usePostSelection()
  const [run, setRun] = createSignal<{ done: number; total: number } | null>(null)

  const items = () => {
    if (!selection?.canSelect() || !selection.selected().has(props.record.uid)) return []
    const selected = new Set([props.record.uid])
    const selectedPosts = ea.selectedPosts(selected)
    if (selectedPosts.length === 0) return []
    const threadUid = props.record.threadUid
    const deleted = ea.deletedCount(selected)
    const live = selected.size - deleted
    const result = [] as Array<{ label: string; danger?: boolean; onClick: () => void }>
    const start = (total: number, action: (onProgress: (done: number, total: number) => void) => Promise<void>) => {
      setRun({ done: 0, total })
      action((done, total) => setRun({ done, total }))
        .then(() => selection.toggle(props.record.uid, false))
        .catch(() => undefined)
        .finally(() => setRun(null))
    }

    if (deleted > 0) {
      result.push({
        label: t('select.restoreSelected', { n: deleted }),
        onClick: () => start(deleted, (onProgress) => ea.runBatchPostRestore(selectedPosts, threadUid, onProgress)),
      })
    }
    if (live > 0) {
      result.push({
        label: t('select.deleteSelected', { n: live }),
        danger: true,
        onClick: () => start(live, (onProgress) => ea.runBatchPostDelete(selectedPosts, threadUid, onProgress)),
      })
    }
    if (ea.canBanAny()) {
      result.push({ label: t('select.banSelected', { n: selected.size }), danger: true, onClick: () => ea.openBan(selectedPosts, threadUid) })
      result.push({
        label: t('select.deleteBanSelected', { n: selected.size }),
        danger: true,
        onClick: () => ea.openBan(selectedPosts, threadUid, true),
      })
    }
    result.push({ label: t('select.clear'), onClick: () => selection.clear() })
    return result
  }

  return (
    <>
      <DropdownMenu
        items={items()}
        triggerSize="xs"
        triggerShape={props.potato ? 'square' : 'rounded'}
        icon={<span class={props.potato ? 'potato-selection-glyph' : 'post-selection-glyph'}>☣</span>}
        class={props.potato ? 'potato-selection-trigger' : 'post-selection-trigger'}
        ariaLabel={t('select.postActions')}
      />
      <Show when={run()}>
        {(progress) => <ProgressPill label={t('select.progress', { done: progress().done, total: progress().total })} />}
      </Show>
    </>
  )
}
