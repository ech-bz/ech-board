import { Show, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import { cva } from 'class-variance-authority'
import { ChevronLeftIcon } from '../icons/ChevronLeftIcon'
import { IconButton } from './IconButton'
import { Capsule } from './Capsule'
import { useI18n, useIsPotato } from '../i18n'
import { usePotatoHead } from '../potatoHead'

const headerVariants = cva('glass-fade sticky top-0 z-nav relative flex h-15 w-full items-center pl-[1.4375rem] pr-[1.375rem] pt-[env(safe-area-inset-top,0px)] max-[899px]:pl-[1.125rem] max-[899px]:pr-[1.125rem]')
const potatoTitleVariants = cva('text-2xl font-bold text-accent [overflow-wrap:anywhere]', {
  variants: { link: { true: 'cursor-pointer hover:text-accent-hover hover:underline', false: '' } },
  defaultVariants: { link: false },
})
const potatoSubVariants = cva('text-sm text-secondary')
const potatoActionsVariants = cva('mt-2 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xl')
const potatoBackVariants = cva('cursor-pointer border-0 bg-transparent p-0 text-accent [font:inherit] hover:text-accent-hover hover:underline')
const capsuleVariants = cva('min-w-0 flex-1')
const rowVariants = cva('grid w-full min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2')
const startVariants = cva('relative z-10 flex shrink-0 items-center justify-start')
const endVariants = cva('relative z-10 flex shrink-0 items-center justify-end')
const titleBlockVariants = cva('flex min-w-0 flex-col items-center justify-center gap-0.5 overflow-hidden px-1')
const headerTitleVariants = cva('w-full min-w-0 truncate text-center text-xl font-semibold leading-none')
const headerSubVariants = cva('w-full min-w-0 truncate text-center text-sm leading-none text-secondary')

export function PageHeader(props: {
  title: JSX.Element
  subtitle?: JSX.Element
  onBack?: () => void
  trailing?: JSX.Element
  below?: JSX.Element
  onTitle?: () => void
}) {
  const { t } = useI18n()
  const potato = useIsPotato()
  const head = usePotatoHead()
  const potatoView = () => (
    <Show when={head()}>
      {(mount) => (
        <Portal mount={mount()}>
          <div data-page-header="">
            <h1
              class={potatoTitleVariants({ link: !!props.onTitle })}
              role={props.onTitle ? 'button' : undefined}
              tabindex={props.onTitle ? 0 : undefined}
              onClick={() => props.onTitle?.()}
              onKeyDown={(e) => {
                if (props.onTitle && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault()
                  props.onTitle()
                }
              }}
            >
              {props.title}
            </h1>
            <Show when={props.subtitle}>
              <div class={potatoSubVariants()}>{props.subtitle}</div>
            </Show>
            <Show when={props.onBack || props.trailing}>
              <div class={potatoActionsVariants()}>
                <Show when={props.onBack}>
                  {(onBack) => (
                    <button type="button" class={potatoBackVariants()} onClick={() => onBack()()}>
                      {t('ui.back')}
                    </button>
                  )}
                </Show>
                {props.trailing}
              </div>
            </Show>
            {props.below}
          </div>
        </Portal>
      )}
    </Show>
  )
  return (
    <Show when={!potato()} fallback={potatoView()}>
    <div data-page-header="" class={headerVariants()}>
      <Capsule class={capsuleVariants()}>
        <div class={rowVariants()}>
          <div class={startVariants()}>
            <Show when={props.onBack}>
              {(onBack) => (
                <IconButton size="lg" aria-label={t('ui.back')} onClick={() => onBack()()}>
                  <ChevronLeftIcon width={24} height={24} />
                </IconButton>
              )}
            </Show>
          </div>
          <div class={titleBlockVariants()}>
            <h1 class={headerTitleVariants()}>{props.title}</h1>
            <Show when={props.subtitle}>
              <div class={headerSubVariants()}>{props.subtitle}</div>
            </Show>
          </div>
          <div class={endVariants()}>{props.trailing}</div>
        </div>
      </Capsule>
      {props.below}
    </div>
    </Show>
  )
}
