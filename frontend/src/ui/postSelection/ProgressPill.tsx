import { cva } from 'class-variance-authority'

const progressVariants = cva('fixed left-1/2 top-16 z-overlay -translate-x-1/2 whitespace-nowrap rounded-full border border-separator bg-active px-3 py-1.5 text-md text-fg')

export function ProgressPill(props: { label: string }) {
  return <div class={progressVariants()}>{props.label}</div>
}
