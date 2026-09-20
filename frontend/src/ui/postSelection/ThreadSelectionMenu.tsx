import { createSignal, Show } from 'solid-js'
import { useI18n } from '../i18n'
import { usePostSelection } from './PostSelectionProvider'
import { useEntityActions } from '../entityActions'
import { ProgressPill } from './ProgressPill'
import { DropdownMenu, type MenuItem } from '../components/Menu/DropdownMenu'

export function ThreadSelectionMenu(props: { items: MenuItem[]; class?: string; variant?: 'glass' }) {
  const { t } = useI18n()
  const sel = usePostSelection()
  const ea = useEntityActions()
  const [run, setRun] = createSignal<{ done: number; total: number } | null>(null)
  const items = (): MenuItem[] => {
    const all = [...props.items]
    if (!sel || !sel.hasSelection()) return all
    const selected = sel.selected()
    const threadUid = ea.threadUid()
    const selectedPosts = ea.selectedPosts(selected)
    const deleted = ea.deletedCount(selected)
    const live = selected.size - deleted
    const start = (total: number, fn: (onProgress: (done: number, total: number) => void) => Promise<void>) => {
      setRun({ done: 0, total })
      fn((done, total) => setRun({ done, total }))
        .then(() => sel.clear())
        .catch(() => undefined)
        .finally(() => setRun(null))
    }
    if (deleted > 0 && threadUid) {
      all.push({
        label: t('select.restoreSelected', { n: deleted }),
        onClick: () => start(deleted, (onProgress) => ea.runBatchPostRestore(selectedPosts, threadUid, onProgress)),
      })
    }
    if (live > 0 && threadUid) {
      all.push({
        label: t('select.deleteSelected', { n: live }),
        danger: true,
        onClick: () => start(live, (onProgress) => ea.runBatchPostDelete(selectedPosts, threadUid, onProgress)),
      })
    }
    if (ea.canBanAny() && threadUid) {
      all.push({ label: t('select.banSelected', { n: selected.size }), danger: true, onClick: () => ea.openBan(selectedPosts, threadUid) })
      all.push({
        label: t('select.deleteBanSelected', { n: selected.size }),
        danger: true,
        onClick: () => ea.openBan(selectedPosts, threadUid, true),
      })
    }
    all.push({ label: t('select.clear'), onClick: () => sel.clear() })
    return all
  }
  return (
    <>
      <DropdownMenu items={items()} class={props.class} triggerVariant={props.variant} />
      <Show when={run()}>
        {(progress) => <ProgressPill label={t('select.progress', { done: progress().done, total: progress().total })} />}
      </Show>
    </>
  )
}
