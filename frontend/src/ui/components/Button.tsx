import { Show, splitProps, type JSX } from 'solid-js'
import { cva, type VariantProps } from 'class-variance-authority'
import { clsx } from 'clsx'

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-1 rounded-md bg-panel2 px-3 py-2 text-md text-accent cursor-pointer border-0 disabled:opacity-45 disabled:cursor-default hover:bg-hover',
  {
    variants: {
      size: {
        sm: 'px-3 py-1.5',
        xs: 'px-2.5 py-1.5',
      },
      intent: {
        accent: 'text-accent',
        danger: 'text-danger',
      },
    },
  },
)

interface Props extends JSX.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {
  href?: string
}

export function Button(props: Props) {
  const [local, rest] = splitProps(props, ['size', 'intent', 'class', 'href'])
  const cls = () => clsx(buttonVariants({ size: local.size, intent: local.intent }), local.class)
  return (
    <Show when={local.href} fallback={<button type="button" class={cls()} {...rest} />}>
      {(href) => <a href={href()} class={cls()} {...(rest as JSX.AnchorHTMLAttributes<HTMLAnchorElement>)} />}
    </Show>
  )
}
