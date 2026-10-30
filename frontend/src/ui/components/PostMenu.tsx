import { useI18n } from '../i18n'
import { DropdownMenu, type MenuItem } from './Menu/DropdownMenu'
import { useEntityActions } from '../entityActions'
import { useStore } from '../store'
import { usePostSelection } from '../postSelection/PostSelectionProvider'
import { threadModItems } from '../threadModItems'
import type { PostRecord } from '../../store/records'
import { canSelectAllFromAuthor, canBanByUid, type BanScope } from '../../core/intent/capabilities'
import type { ThreadObject } from '../../core/bcs/types'

export function PostMenu(props: { record: PostRecord; closed: boolean; threadObject?: ThreadObject; onEdit?: (uid: string) => void; onOpenThread?: () => void; potato?: boolean; label?: string }) {
  const { t } = useI18n()
  const actions = useEntityActions()
  const store = useStore()
  const selection = usePostSelection()
  const items = (): MenuItem[] => {
    const rec = props.record
    const result: MenuItem[] = []
    if (props.onEdit && rec.canEdit) result.push({ label: t('mod.edit'), onClick: () => props.onEdit?.(rec.uid) })
    const author = (): MenuItem => ({ label: t('mod.selectAllFromAuthor'), onClick: () => selection?.selectAllFromAuthor(rec.post, rec.threadUid) })
    if (rec.op) {
      if (props.onOpenThread) result.push({ label: t('thread.open'), onClick: props.onOpenThread })
      result.push(...threadModItems(t, actions, { threadUid: rec.threadUid, closed: props.closed, deleted: rec.deleted }))
      result.push({ label: t('mod.moderators'), onClick: () => actions.openThreadModerators(rec.threadUid) })
      result.push({ label: t('mod.bans'), onClick: () => actions.openBans(t('mod.bans'), rec.threadUid) })
      if (actions.canBanAny() && canBanByUid(actions.roleKinds())) result.push({ label: t('mod.banUid'), onClick: () => actions.openBanUid() })
      if (selection?.canSelect() && canSelectAllFromAuthor(actions.roleKinds())) result.push(author())
      result.push({ label: t('mod.debugThread'), onClick: () => actions.openLogs('thread', props.threadObject) })
      result.push({ label: t('mod.debugPost'), onClick: () => actions.openLogs('post', rec.post) })
      return result
    }
    const threadUid = rec.threadUid
    if (rec.canDelete) result.push({ label: t('mod.delete'), onClick: () => actions.runPostDelete(rec.post, threadUid, 'delete') })
    if (rec.canRestore) result.push({ label: t('mod.restore'), onClick: () => actions.runPostDelete(rec.post, threadUid, 'restore') })
    if (actions.canBanAny() && !rec.deleted && !rec.banned) {
      result.push({ label: t('mod.ban'), onClick: () => actions.openBan([rec.post], threadUid) })
    }
    if (rec.banned && rec.bannedLevel) {
      const info = store.info()
      const level = rec.bannedLevel
      const scope: BanScope = level === threadUid ? 'thread' : level === info?.boardUid ? 'board' : 'forum'
      if (actions.canBanScope(scope) && info) {
        result.push({ label: t('mod.unban'), onClick: () => actions.runUnban(rec.post, threadUid, scope) })
      }
    }
    if (selection?.canSelect() && canSelectAllFromAuthor(actions.roleKinds())) result.push(author())
    result.push({ label: t('mod.debugPost'), onClick: () => actions.openLogs('post', rec.post) })
    return result
  }
  return (
    <DropdownMenu
      items={items()}
      triggerSize="xs"
      iconSize={14}
      triggerShape={props.potato ? 'square' : 'rounded'}
      icon={props.potato ? <span class="potato-menu-glyph">▶</span> : undefined}
      class={props.potato ? 'potato-menu-trigger' : 'ml-auto self-start'}
      label={props.potato ? undefined : props.label}
    />
  )
}
