import { cva } from 'class-variance-authority'
import { createSignal } from 'solid-js'

const checkWrapVariants = cva('relative flex flex-none cursor-pointer items-center')
const checkVariants = cva('relative size-4 cursor-pointer appearance-none rounded-sm border border-panel3 bg-bg checked:border-accent checked:bg-accent')

export function ModCheck(props: { checked: boolean; onToggle: () => void; label?: string }) {
  return (
    <span role="checkbox" aria-checked={props.checked} aria-label={props.label} class={checkWrapVariants()} onClick={props.onToggle}>
      <input type="checkbox" class={`${checkVariants()} post-select-check`} checked={props.checked} readOnly />
    </span>
  )
}

export function createSelection<K>() {
  const [selected, setSelected] = createSignal<Set<K>>(new Set<K>())
  const toggle = (key: K) =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  const clear = () => setSelected(new Set<K>())
  return { selected, has: (key: K) => selected().has(key), toggle, clear, size: () => selected().size }
}
