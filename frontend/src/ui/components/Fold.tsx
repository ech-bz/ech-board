import { createContext, createEffect, createSignal, createUniqueId, on, useContext, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'

const FoldContext = createContext<{ depth: number; parentOpen: () => boolean }>({ depth: 0, parentOpen: () => true })

const foldVariants = cva('relative inline-block w-full align-top my-0.5 rounded-sm pl-2.5 overflow-hidden', {
  variants: { depth: { even: 'bg-panel2', odd: 'bg-[var(--post-bg)]' } },
})
const foldRailVariants = cva('fold-rail absolute top-0 bottom-0 left-0 w-0.5 rounded-full bg-accent/80 transition-all duration-200')
const foldTriggerVariants = cva(
  'grid min-h-8 w-full cursor-pointer grid-cols-[minmax(0,1fr)_16px] items-center rounded-sm border-0 bg-transparent py-0 pr-1.5 pl-2 text-left font-semibold text-fg text-xl',
)
const foldChevronVariants = cva('h-2 w-2 justify-self-center border-r border-b transition duration-200', {
  variants: {
    open: { false: '-translate-y-0.5 rotate-45 border-secondary', true: 'translate-y-0.5 rotate-[225deg] border-accent' },
  },
})
const foldTitleVariants = cva('break-words')
const foldPanelVariants = cva('grid transition-[grid-template-rows,opacity] duration-200', {
  variants: { open: { false: 'grid-rows-[0fr] opacity-0', true: 'grid-rows-[1fr] opacity-100' } },
})
const foldPanelInnerVariants = cva('min-h-0 overflow-hidden')
const foldBodyVariants = cva('py-0.5 pr-6 pl-2 text-base text-fg')

export function Fold(props: { title: string; children: JSX.Element }) {
  const { expandFolds } = useI18n()
  const parent = useContext(FoldContext)
  const [open, setOpen] = createSignal(expandFolds())
  createEffect(
    on(
      expandFolds,
      (value) => {
        if (value) setOpen(true)
      },
      { defer: true },
    ),
  )
  createEffect(
    on(
      parent.parentOpen,
      (value) => {
        if (!value) setOpen(false)
      },
      { defer: true },
    ),
  )
  const panelId = createUniqueId()
  return (
    <div class={foldVariants({ depth: parent.depth % 2 === 0 ? 'even' : 'odd' })}>
      <span class={foldRailVariants()} aria-hidden="true" />
      <button
        type="button"
        class={foldTriggerVariants()}
        aria-expanded={open()}
        aria-controls={panelId}
        onClick={() => setOpen(!open())}
      >
        <span class={foldTitleVariants()}>{props.title}</span>
        <span class={foldChevronVariants({ open: open() })} aria-hidden="true" />
      </button>
      <div id={panelId} class={foldPanelVariants({ open: open() })} inert={!open()} data-fold-title={props.title}>
        <div class={foldPanelInnerVariants()}>
          <div class={foldBodyVariants()}>
            <FoldContext.Provider value={{ depth: parent.depth + 1, parentOpen: open }}>{props.children}</FoldContext.Provider>
          </div>
        </div>
      </div>
    </div>
  )
}
