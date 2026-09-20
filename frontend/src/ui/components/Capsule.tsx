import { splitProps, type JSX } from 'solid-js'
import { cva, type VariantProps } from 'class-variance-authority'
import { clsx } from 'clsx'

const capsuleVariants = cva('flex items-center justify-center bg-glass2 rounded-full glass', {
  variants: {
    size: {
      sm: 'h-9',
      md: 'h-10',
      lg: 'h-11',
    },
    visible: {
      true: 'pointer-events-auto opacity-100',
    },
  },
  defaultVariants: { size: 'lg' },
})

interface Props extends JSX.HTMLAttributes<HTMLDivElement>, VariantProps<typeof capsuleVariants> {}

export function Capsule(props: Props) {
  const [local, rest] = splitProps(props, ['size', 'visible', 'class'])
  return <div class={clsx(capsuleVariants({ size: local.size, visible: local.visible }), local.class)} {...rest} />
}
