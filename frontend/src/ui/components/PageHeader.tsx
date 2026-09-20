import { Show, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { ChevronLeftIcon } from '../icons/ChevronLeftIcon'
import { IconButton } from './IconButton'
import { Capsule } from './Capsule'

const headerVariants = cva('glass-fade sticky top-0 z-nav flex min-h-header-mobile items-stretch gap-2 px-4 pb-2.5 pt-[calc(12px+env(safe-area-inset-top,0px))]')
const headerTitleVariants = cva('w-full min-w-0 ellipsis text-center text-xl font-semibold leading-none')
const headerSubVariants = cva('w-full min-w-0 ellipsis overflow-y-visible text-center text-sm text-secondary leading-none')
const backBtnVariants = cva('text-4xl leading-none')

export function PageHeader(props: {
  title: JSX.Element
  subtitle?: JSX.Element
  onBack?: () => void
  trailing?: JSX.Element
  below?: JSX.Element
}) {
  return (
    <div data-page-header="" class={headerVariants()}>
      <Show when={props.onBack}>
        {(onBack) => (
          <IconButton size="lg" variant="glass" class={backBtnVariants()} onClick={() => onBack()()}>
            <ChevronLeftIcon width={24} height={24} />
          </IconButton>
        )}
      </Show>
      <Capsule class="flex-1 min-w-0 flex-col gap-0.5 px-3">
        <h1 class={headerTitleVariants()}>{props.title}</h1>
        <Show when={props.subtitle}>
          <div class={headerSubVariants()}>{props.subtitle}</div>
        </Show>
      </Capsule>
      {props.trailing}
      {props.below}
    </div>
  )
}
