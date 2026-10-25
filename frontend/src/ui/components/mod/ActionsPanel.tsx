import { createMemo, createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ModWindow } from './ModWindow'
import { matchesQuery, useModFormat } from './format'
import { IconButton } from '../IconButton'
import { DropdownMenu, type MenuItem } from '../Menu/DropdownMenu'
import { ModCheck, createSelection } from './ModCheck'
import { UndoIcon } from '../../icons/UndoIcon'
import { useI18n } from '../../i18n'
import { useNav } from '../../nav'
import { useIsMobile } from '../Gallery/useIsMobile'
import { threadPath } from '../../route'
import { modData } from '../../mod/modData'
import type { ModActionEntry, ModActionKind } from '../../mod/types'
import type { MessageKey } from '../../i18n/ru'

const tableVariants = cva('w-full border-separate border-spacing-0 text-lg')
const thVariants = cva(
  'sticky top-0 z-control whitespace-nowrap border-b border-r border-panel3/60 bg-panel2 px-3 py-2.5 text-center font-medium text-secondary last:border-r-0',
)
const tdVariants = cva('border-b border-r border-panel3/60 px-3 py-2 align-middle last:border-r-0')
const rowVariants = cva('hover:bg-hover', { variants: { undone: { true: 'opacity-55' }, selected: { true: 'bg-active' } } })
const uidVariants = cva('block max-w-[16ch] truncate whitespace-nowrap text-secondary')
const numVariants = cva('post-num cursor-pointer whitespace-nowrap font-semibold text-accent no-underline')
const subVariants = cva('block text-md text-secondary')
const emptyVariants = cva('p-8 text-center text-secondary')
const cardVariants = cva('flex items-start gap-2 border-b border-panel3/60 px-3 py-2.5', { variants: { selected: { true: 'bg-active' } } })

const ACTION_KEY: Record<ModActionKind, MessageKey> = {
  delete: 'modUi.act.delete',
  restore: 'modUi.act.restore',
  ban: 'modUi.act.ban',
  hide_media: 'modUi.act.hideMedia',
  delete_file: 'modUi.act.deleteFile',
  close: 'modUi.act.close',
  pin: 'modUi.act.pin',
}

function useAction(props: { entry: ModActionEntry; forumId: string }) {
  const nav = useNav()
  const e = () => props.entry
  const open = () => {
    if (e().board && e().threadNum != null && e().postNum != null) {
      nav.navigate({ page: 'thread', forumId: props.forumId, slug: e().board!, threadNum: e().threadNum!, postNum: e().postNum! })
    }
  }
  const href = () => (e().board && e().threadNum != null ? threadPath(props.forumId, e().board!, e().threadNum!, e().postNum!) : undefined)
  return { e, open, href }
}

function UndoButton(props: { entry: ModActionEntry; canModerate: boolean }) {
  const { t } = useI18n()
  return (
    <Show when={props.canModerate && !props.entry.undone}>
      <IconButton size="sm" shape="rounded" aria-label={t('modUi.undo')} title={t('modUi.undo')} class="hover:bg-hover" onClick={() => modData.undoAction(props.entry.id)}>
        <UndoIcon width={14} height={14} />
      </IconButton>
    </Show>
  )
}

type RowProps = { entry: ModActionEntry; forumId: string; canModerate: boolean; selected: boolean; onToggle: () => void }

function ActionRow(props: RowProps) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const { e, open, href } = useAction(props)
  return (
    <tr class={rowVariants({ undone: e().undone, selected: props.selected })}>
      <td class={tdVariants()}>
        <div class="flex items-center gap-2">
          <Show when={props.canModerate}>
            <ModCheck checked={props.selected} onToggle={props.onToggle} />
          </Show>
          <span class={uidVariants()} title={e().modUid}>
            {e().modUid}
          </span>
        </div>
      </td>
      <td class={tdVariants()}>
        <span class="font-medium">{t(ACTION_KEY[e().kind])}</span>
        <span class={subVariants()}>
          {t(`modUi.target.${e().target}` as MessageKey)}
          {e().detail ? ` ${e().detail}` : ''}
          {e().undone ? ` · ${t('modUi.undone')}` : ''}
        </span>
      </td>
      <td class={tdVariants()}>{fmt.place(e())}</td>
      <td class={`${tdVariants()} text-center`}>
        <Show when={e().postNum != null}>
          <a class={numVariants()} href={href()} onClick={(ev) => (ev.preventDefault(), open())}>
            #{e().postNum}
          </a>
        </Show>
      </td>
      <td class={`${tdVariants()} text-center`}>
        <span class="whitespace-nowrap">{fmt.date(e().ts)}</span>
        <span class={subVariants()}>{fmt.time(e().ts)}</span>
      </td>
      <td class={`${tdVariants()} w-8 text-center`}>
        <UndoButton entry={e()} canModerate={props.canModerate} />
      </td>
    </tr>
  )
}

function ActionCard(props: RowProps) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const { e, open, href } = useAction(props)
  return (
    <article class={`${cardVariants({ selected: props.selected })} ${e().undone ? 'opacity-55' : ''}`}>
      <Show when={props.canModerate}>
        <span class="pt-0.5">
          <ModCheck checked={props.selected} onToggle={props.onToggle} />
        </span>
      </Show>
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-baseline gap-x-2 text-lg">
          <span class="font-medium">{t(ACTION_KEY[e().kind])}</span>
          <span class="text-md text-secondary">
            {t(`modUi.target.${e().target}` as MessageKey)}
            {e().detail ? ` ${e().detail}` : ''}
            {e().undone ? ` · ${t('modUi.undone')}` : ''}
          </span>
        </div>
        <div class="mt-0.5 flex flex-wrap items-center gap-x-2 text-md text-secondary">
          <span>{fmt.place(e())}</span>
          <Show when={e().postNum != null}>
            <a class={numVariants()} href={href()} onClick={(ev) => (ev.preventDefault(), open())}>
              #{e().postNum}
            </a>
          </Show>
        </div>
        <div class="mt-0.5 flex items-center gap-2 text-md text-secondary">
          <span class={uidVariants()}>{e().modUid}</span>
          <span class="whitespace-nowrap">{fmt.full(e().ts)}</span>
        </div>
      </div>
      <UndoButton entry={e()} canModerate={props.canModerate} />
    </article>
  )
}

export function ActionsPanel(props: { forumId: string; canModerate: boolean }) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const mobile = useIsMobile()
  const [query, setQuery] = createSignal('')
  const sel = createSelection<string>()
  const rows = createMemo(() =>
    modData
      .actions()
      .filter((a) => matchesQuery(query(), a.modUid, t(ACTION_KEY[a.kind]), t(`modUi.target.${a.target}` as MessageKey), a.detail, fmt.place(a), a.postNum)),
  )
  const bulkItems = (): MenuItem[] => {
    const todo = modData.actions().filter((a) => sel.has(a.id) && !a.undone)
    const items: MenuItem[] = []
    if (todo.length) items.push({ label: t('select.undoSelected', { n: todo.length }), danger: true, onClick: () => (todo.forEach((a) => modData.undoAction(a.id)), sel.clear()) })
    items.push({ label: t('select.clear'), onClick: sel.clear })
    return items
  }
  const rowProps = (entry: ModActionEntry) => ({
    entry,
    forumId: props.forumId,
    canModerate: props.canModerate,
    selected: sel.has(entry.id),
    onToggle: () => sel.toggle(entry.id),
  })
  return (
    <ModWindow
      framed
      search={query()}
      onSearch={setQuery}
      trailing={
        <Show when={sel.size() > 0}>
          <DropdownMenu
            items={bulkItems()}
            ariaLabel={t('modUi.selected', { n: sel.size() })}
            icon={<span class="text-md font-semibold">{sel.size()}</span>}
            triggerSize="md"
            triggerVariant="glass"
          />
        </Show>
      }
    >
      <Show when={rows().length > 0} fallback={<div class={emptyVariants()}>{t('modUi.empty')}</div>}>
        <Show
          when={!mobile()}
          fallback={<For each={rows()}>{(entry) => <ActionCard {...rowProps(entry)} />}</For>}
        >
          <table class={tableVariants()}>
            <thead>
              <tr>
                <th class={thVariants()}>{t('modUi.moderator')}</th>
                <th class={thVariants()}>{t('modUi.action')}</th>
                <th class={thVariants()}>{t('modUi.place')}</th>
                <th class={thVariants()}>{t('modUi.post')}</th>
                <th class={thVariants()}>{t('modUi.time')}</th>
                <th class={thVariants()} />
              </tr>
            </thead>
            <tbody class="[&_tr:last-child>td]:border-b-0">
              <For each={rows()}>{(entry) => <ActionRow {...rowProps(entry)} />}</For>
            </tbody>
          </table>
        </Show>
      </Show>
    </ModWindow>
  )
}
