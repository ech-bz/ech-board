import { splitProps, type JSX } from 'solid-js'
import { cva, type VariantProps } from 'class-variance-authority'
import { clsx } from 'clsx'

const iconButtonVariants = cva(
  'flex-none inline-flex items-center justify-center text-fg border-0 cursor-pointer disabled:opacity-45 disabled:cursor-default',
  {
    variants: {
      size: {
        xs: 'size-4',
        sm: 'size-6',
        md: 'size-9',
        lg: 'size-11',
        xl: 'size-12',
      },
      shape: {
        circle: 'rounded-full',
        rounded: 'rounded-md',
        square: 'rounded-none',
      },
      variant: {
        glass: 'bg-glass2 glass',
      },
      active: {
        true: 'text-accent',
      },
    },
    defaultVariants: { shape: 'circle' },
  },
)

interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof iconButtonVariants> {}

export function IconButton(props: Props) {
  const [local, rest] = splitProps(props, ['size', 'shape', 'variant', 'active', 'class'])
  return (
    <button
      type="button"
      class={clsx(
        iconButtonVariants({ size: local.size, shape: local.shape, variant: local.variant, active: local.active }),
        local.class,
      )}
      {...rest}
    />
  )
}
