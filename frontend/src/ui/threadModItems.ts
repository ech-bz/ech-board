import type { MenuItem } from './components/Menu/DropdownMenu'
import type { EntityActions } from './entityActions'
import type { MessageKey } from './i18n/ru'

export function threadModItems(
  t: (key: MessageKey) => string,
  ea: EntityActions,
  u: { threadUid: string; closed: boolean; deleted: boolean },
): MenuItem[] {
  const items: MenuItem[] = []
  const { threadUid, closed, deleted } = u
  if (deleted) {
    if (ea.canThreadRestore()) items.push({ label: t('mod.restoreThread'), onClick: () => ea.runThreadAction('restore', threadUid) })
  } else {
    if (closed) {
      if (ea.canThreadOpen()) items.push({ label: t('mod.open'), onClick: () => ea.runThreadAction('open', threadUid) })
      if (ea.canThreadDelete()) items.push({ label: t('mod.deleteThread'), onClick: () => ea.runThreadAction('delete', threadUid) })
    } else {
      if (ea.canThreadClose()) items.push({ label: t('mod.close'), onClick: () => ea.runThreadAction('close', threadUid) })
      if (ea.canThreadDelete()) items.push({ label: t('mod.closeDelete'), onClick: () => ea.runThreadCloseDelete(threadUid) })
    }
    if (ea.canThreadEditTopic()) items.push({ label: t('mod.editTopic'), onClick: () => ea.openThreadTopic(threadUid) })
  }
  if (!deleted && ea.canThreadPin()) {
    items.push({
      label: t(ea.isThreadPinned(threadUid) ? 'mod.unpin' : 'mod.pin'),
      onClick: () => ea.toggleThreadPin(threadUid),
    })
  }
  return items
}
