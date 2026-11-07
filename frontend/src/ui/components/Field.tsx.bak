import { splitProps, type JSX } from 'solid-js'
import { cva, type VariantProps } from 'class-variance-authority'
import { clsx } from 'clsx'

const fieldVariants = cva('bg-panel2 text-fg text-md border-0 outline-none px-3 py-2 rounded-md appearance-none', {
  variants: {
    bordered: {
      true: 'border border-separator',
    },
    select: {
      true: 'px-2.5 py-2 text-lg field-sizing-content',
    },
    grow: {
      'min-w-input': 'min-w-input flex-1',
      'min-w-0': 'min-w-0 flex-1',
      'basis-full': 'min-w-0 basis-full flex-1',
    },
  },
})

type FieldVariantProps = VariantProps<typeof fieldVariants>

interface CommonProps extends FieldVariantProps {
  class?: string
}

type Props =
  | (JSX.InputHTMLAttributes<HTMLInputElement> & CommonProps & { as?: 'input' })
  | (JSX.SelectHTMLAttributes<HTMLSelectElement> & CommonProps & { as: 'select' })

export function Field(props: Props) {
  const [local, rest] = splitProps(props, ['as', 'bordered', 'select', 'grow', 'class'])
  const cls = () =>
    clsx(fieldVariants({ bordered: local.bordered, select: local.select, grow: local.grow }), local.class)
  if (local.as === 'select') {
    return <select class={cls()} {...(rest as JSX.SelectHTMLAttributes<HTMLSelectElement>)} />
  }
  return <input class={cls()} {...(rest as JSX.InputHTMLAttributes<HTMLInputElement>)} />
}
