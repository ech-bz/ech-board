import { Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ArrowDownIcon } from '../icons/ArrowDownIcon'
import { ArrowUpIcon } from '../icons/ArrowUpIcon'
import { useI18n } from '../i18n'
import { IconButton } from './IconButton'

const upBtnVariants = cva('absolute right-4 top-full z-50')
const downBtnVariants = cva('absolute right-4 bottom-full z-50')
const unreadBadgeVariants = cva('absolute -right-1.5 -top-1.5 flex h-badge min-w-badge items-center justify-center rounded-md bg-danger px-1.5 text-center text-sm font-bold text-white')

export function ThreadNavUp(props: { onClick: () => void }) {
  const { t } = useI18n()
  return (
    <IconButton size="lg" variant="glass" class={upBtnVariants()} aria-label={t('thread.up')} onClick={props.onClick}>
      <ArrowUpIcon width={20} height={20} />
    </IconButton>
  )
}

export function ThreadNavDown(props: { unread: number; onClick: () => void }) {
  const { t } = useI18n()
  return (
    <IconButton size="lg" variant="glass" class={downBtnVariants()} aria-label={t('thread.down')} onClick={props.onClick}>
      <ArrowDownIcon width={20} height={20} />
      <Show when={props.unread > 0}>
        <span class={unreadBadgeVariants()}>{props.unread}</span>
      </Show>
    </IconButton>
  )
}
