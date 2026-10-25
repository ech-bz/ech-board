import type { JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Field } from '../Field'
import { useI18n } from '../../i18n'

const windowVariants = cva('flex min-h-0 min-w-0 flex-1 flex-col gap-2')
const headVariants = cva('flex min-h-9 flex-none items-center justify-end gap-2')
const bodyVariants = cva('min-h-0 flex-1 overflow-y-auto overscroll-contain rounded-lg bg-panel1', {
  variants: { framed: { true: 'border border-panel3/60' } },
})

export function ModWindow(props: {
  search: string
  onSearch: (value: string) => void
  trailing?: JSX.Element
  scrollRef?: (el: HTMLDivElement) => void
  class?: string
  framed?: boolean
  children: JSX.Element
}) {
  const { t } = useI18n()
  return (
    <section class={`${windowVariants()} ${props.class ?? ''}`}>
      <div class={headVariants()}>
        {props.trailing}
        <Field
          type="search"
          bordered={props.framed}
          class="min-w-0 max-w-56 flex-1 max-[899px]:max-w-none"
          placeholder={t('modUi.search')}
          aria-label={t('modUi.search')}
          value={props.search}
          onInput={(e) => props.onSearch(e.currentTarget.value)}
        />
      </div>
      <div ref={props.scrollRef} class={bodyVariants({ framed: props.framed })}>
        {props.children}
      </div>
    </section>
  )
}
