import { Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ArrowDownIcon } from '../icons/ArrowDownIcon'
import { ArrowUpIcon } from '../icons/ArrowUpIcon'
import { useI18n } from '../i18n'
import { IconButton } from './IconButton'

const upBtnVariants = cva('nav-up absolute right-[1.125rem] min-[900px]:right-[1.375rem] top-full z-50 bg-chip transition-colors hover:bg-hover')
const downBtnVariants = cva('absolute right-2 bottom-[calc(100%+4px)] z-50 bg-chip transition-colors hover:bg-hover')
const unreadBadgeVariants = cva('absolute -right-1.5 -top-1.5 flex h-badge min-w-badge items-center justify-center rounded-md bg-danger px-1.5 text-center text-sm font-bold text-white')

export function ThreadNavUp(props: { onClick: () => void }) {
  const { t } = useI18n()
  return (
    <IconButton size="lg" class={upBtnVariants()} aria-label={t('thread.up')} onClick={props.onClick}>
      <ArrowUpIcon width={20} height={20} />
    </IconButton>
  )
}

export function ThreadNavDown(props: { unread: number; onClick: () => void }) {
  const { t } = useI18n()
  return (
    <IconButton size="lg" class={downBtnVariants()} aria-label={t('thread.down')} onClick={props.onClick}>
      <ArrowDownIcon width={20} height={20} />
      <Show when={props.unread > 0}>
        <span class={unreadBadgeVariants()}>{props.unread}</span>
      </Show>
    </IconButton>
  )
}