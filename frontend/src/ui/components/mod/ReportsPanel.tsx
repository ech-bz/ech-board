import { createMemo, createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ModWindow } from './ModWindow'
import { matchesQuery, useModFormat } from './format'
import { DropdownMenu, type MenuItem } from '../Menu/DropdownMenu'
import { Field } from '../Field'
import { ModCheck, createSelection } from './ModCheck'
import { useI18n } from '../../i18n'
import { useNav } from '../../nav'
import { useIsMobile } from '../Gallery/useIsMobile'
import { threadPath } from '../../route'
import { modData } from '../../mod/modData'
import type { ModReport, ReportStatus } from '../../mod/types'
import type { MessageKey } from '../../i18n/ru'

const tableVariants = cva('w-full border-separate border-spacing-0 text-lg')
const thVariants = cva(
  'sticky top-0 z-control whitespace-nowrap border-b border-r border-panel3/60 bg-panel2 px-3 py-2.5 text-center font-medium text-secondary last:border-r-0',
)
const tdVariants = cva('border-b border-r border-panel3/60 px-3 py-2 align-middle last:border-r-0')
const rowVariants = cva('hover:bg-hover', { variants: { closed: { true: 'opacity-60' }, selected: { true: 'bg-active' } } })
const uidVariants = cva('block max-w-[16ch] truncate whitespace-nowrap text-secondary')
const numVariants = cva('post-num cursor-pointer whitespace-nowrap font-semibold text-accent no-underline')
const subVariants = cva('block text-md text-secondary')
const statusVariants = cva('whitespace-nowrap text-lg font-semibold', {
  variants: {
    status: { open: 'text-warn', resolved: 'text-success', ignored: 'text-secondary' },
  },
})
const emptyVariants = cva('p-8 text-center text-secondary')
const cardVariants = cva('border-b border-panel3/60 px-3 py-2.5', { variants: { closed: { true: 'opacity-60' }, selected: { true: 'bg-active' } } })

type Filter = ReportStatus | 'all'
const STATUS_KEY: Record<ReportStatus, MessageKey> = {
  open: 'modUi.state.open',
  resolved: 'modUi.state.resolved',
  ignored: 'modUi.state.ignored',
}

function useReport(props: { report: ModReport; forumId: string }) {
  const { t } = useI18n()
  const nav = useNav()
  const r = () => props.report
  const openPost = () => nav.navigate({ page: 'thread', forumId: props.forumId, slug: r().board, threadNum: r().threadNum, postNum: r().postNum })
  const menu = (): MenuItem[] => {
    const items: MenuItem[] = []
    if (r().status === 'open') {
      items.push({ label: t('modUi.resolve'), onClick: () => modData.setReportStatus(r().id, 'resolved') })
      items.push({ label: t('modUi.ignore'), onClick: () => modData.setReportStatus(r().id, 'ignored') })
    } else {
      items.push({ label: t('modUi.reopen'), onClick: () => modData.setReportStatus(r().id, 'open') })
    }
    items.push({ label: t('modUi.openInThread'), onClick: openPost })
    items.push({
      label: t('modUi.deletePost'),
      danger: true,
      onClick: () => {
        modData.deletePosts([r().postNum])
        modData.setReportStatus(r().id, 'resolved')
      },
    })
    return items
  }
  return { r, openPost, menu, href: () => threadPath(props.forumId, r().board, r().threadNum, r().postNum) }
}

type RowProps = { report: ModReport; forumId: string; canModerate: boolean; selected: boolean; onToggle: () => void }

function ReportRow(props: RowProps) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const { r, openPost, menu, href } = useReport(props)
  return (
    <tr class={rowVariants({ closed: r().status !== 'open', selected: props.selected })}>
      <td class={tdVariants()}>
        <div class="flex items-center gap-2">
          <Show when={props.canModerate}>
            <ModCheck checked={props.selected} onToggle={props.onToggle} />
          </Show>
          <span class={uidVariants()} title={r().reporterUid}>
            {r().reporterUid}
          </span>
        </div>
      </td>
      <td class={tdVariants()}>{r().reason}</td>
      <td class={`${tdVariants()} text-center`}>
        <a class={numVariants()} href={href()} onClick={(e) => (e.preventDefault(), openPost())}>
          #{r().postNum}
        </a>
        <span class={subVariants()}>/{r().board}/</span>
      </td>
      <td class={`${tdVariants()} text-center`}>
        <span class="whitespace-nowrap">{fmt.date(r().ts)}</span>
        <span class={subVariants()}>{fmt.time(r().ts)}</span>
      </td>
      <td class={`${tdVariants()} text-center`}>
        <span class={statusVariants({ status: r().status })}>{t(STATUS_KEY[r().status])}</span>
      </td>
      <td class={`${tdVariants()} w-8 text-center`}>
        <Show when={props.canModerate}>
          <DropdownMenu items={menu()} triggerSize="xs" iconSize={14} triggerShape="rounded" />
        </Show>
      </td>
    </tr>
  )
}

function ReportCard(props: RowProps) {
  const { t } = useI18n()
  const fmt = useModFormat()
  const { r, openPost, menu, href } = useReport(props)
  return (
    <article class={cardVariants({ closed: r().status !== 'open', selected: props.selected })}>
      <div class="flex items-center gap-2 text-md">
        <Show when={props.canModerate}>
          <ModCheck checked={props.selected} onToggle={props.onToggle} />
        </Show>
        <a class={numVariants()} href={href()} onClick={(e) => (e.preventDefault(), openPost())}>
          #{r().postNum}
        </a>
        <span class="text-accent">/{r().board}/</span>
        <span class="ml-auto whitespace-nowrap text-sm text-secondary">{fmt.full(r().ts)}</span>
        <Show when={props.canModerate}>
          <DropdownMenu items={menu()} triggerSize="sm" iconSize={16} triggerShape="rounded" />
        </Show>
      </div>
      <div class="mt-1 text-lg">{r().reason}</div>
      <div class="mt-1 flex items-center gap-2">
        <span class={uidVariants()}>{r().reporterUid}</span>
        <span class={statusVariants({ status: r().status })}>{t(STATUS_KEY[r().status])}</span>
      </div>
    </article>
  )
}

export function ReportsPanel(props: { forumId: string; canModerate: boolean }) {
  const { t } = useI18n()
  const mobile = useIsMobile()
  const [query, setQuery] = createSignal('')
  const [filter, setFilter] = createSignal<Filter>('open')
  const sel = createSelection<string>()
  const rows = createMemo(() =>
    modData
      .reports()
      .filter((r) => (filter() === 'all' || r.status === filter()) && matchesQuery(query(), r.reason, r.reporterUid, r.postNum, r.board)),
  )
  const chosen = () => modData.reports().filter((r) => sel.has(r.id))
  const bulkItems = (): MenuItem[] => {
    const all = chosen()
    const open = all.filter((r) => r.status === 'open')
    const closed = all.filter((r) => r.status !== 'open')
    const setAll = (list: ModReport[], status: ReportStatus) => (list.forEach((r) => modData.setReportStatus(r.id, status)), sel.clear())
    const items: MenuItem[] = []
    if (open.length) {
      items.push({ label: t('select.resolveSelected', { n: open.length }), onClick: () => setAll(open, 'resolved') })
      items.push({ label: t('select.ignoreSelected', { n: open.length }), onClick: () => setAll(open, 'ignored') })
    }
    if (closed.length) items.push({ label: t('select.reopenSelected', { n: closed.length }), onClick: () => setAll(closed, 'open') })
    items.push({
      label: t('select.deleteSelected', { n: all.length }),
      danger: true,
      onClick: () => {
        modData.deletePosts([...new Set(all.map((r) => r.postNum))])
        setAll(all, 'resolved')
      },
    })
    items.push({ label: t('select.clear'), onClick: sel.clear })
    return items
  }
  const rowProps = (report: ModReport) => ({
    report,
    forumId: props.forumId,
    canModerate: props.canModerate,
    selected: sel.has(report.id),
    onToggle: () => sel.toggle(report.id),
  })
  return (
    <ModWindow
      framed
      search={query()}
      onSearch={setQuery}
      trailing={
        <>
          <Show when={sel.size() > 0}>
            <DropdownMenu
              items={bulkItems()}
              ariaLabel={t('modUi.selected', { n: sel.size() })}
              icon={<span class="text-md font-semibold">{sel.size()}</span>}
              triggerSize="md"
              triggerVariant="glass"
            />
          </Show>
        <Field as="select" bordered aria-label={t('modUi.status')} class="cursor-pointer font-medium" value={filter()} onInput={(e) => setFilter(e.currentTarget.value as Filter)}>
          <option value="open">{t('modUi.statusOpen')}</option>
          <option value="resolved">{t('modUi.statusResolved')}</option>
          <option value="ignored">{t('modUi.statusIgnored')}</option>
          <option value="all">{t('modUi.statusAll')}</option>
        </Field>
        </>
      }
    >
      <Show when={rows().length > 0} fallback={<div class={emptyVariants()}>{t('modUi.empty')}</div>}>
        <Show
          when={!mobile()}
          fallback={<For each={rows()}>{(report) => <ReportCard {...rowProps(report)} />}</For>}
        >
          <table class={tableVariants()}>
            <thead>
              <tr>
                <th class={thVariants()}>{t('modUi.reporter')}</th>
                <th class={thVariants()}>{t('modUi.reason')}</th>
                <th class={thVariants()}>{t('modUi.post')}</th>
                <th class={thVariants()}>{t('modUi.time')}</th>
                <th class={thVariants()}>{t('modUi.status')}</th>
                <th class={thVariants()} />
              </tr>
            </thead>
            <tbody class="[&_tr:last-child>td]:border-b-0">
              <For each={rows()}>{(report) => <ReportRow {...rowProps(report)} />}</For>
            </tbody>
          </table>
        </Show>
      </Show>
    </ModWindow>
  )
}
