import { For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Button } from '../Button'

export interface MenuItem {
  label: string
  href?: string
  onClick?: () => void
  danger?: boolean
}

const menuItemVariants = cva('w-full cursor-pointer bg-transparent px-2 py-1.5 text-left text-md text-fg rounded-md hover:bg-hover', {
  variants: {
    danger: {
      true: 'text-danger',
    },
  },
})

const menuItemsRowVariants = cva('flex flex-wrap items-center gap-2')

function MenuItemButton(props: { item: MenuItem; onDone: () => void; kind: 'button' | 'item' }) {
  return (
    <Show
      when={props.kind === 'item'}
      fallback={
        <Button
          size="xs"
          intent={props.item.danger ? 'danger' : undefined}
          onClick={() => {
            props.onDone()
            props.item.onClick?.()
          }}
        >
          {props.item.label}
        </Button>
      }
    >
      <Show
        when={props.item.href}
        fallback={
          <button
            type="button"
            class={menuItemVariants({ danger: props.item.danger })}
            onClick={() => {
              props.onDone()
              props.item.onClick?.()
            }}
          >
            {props.item.label}
          </button>
        }
      >
        {(href) => (
          <a href={href()} target="_blank" rel="noopener noreferrer" class={menuItemVariants({ danger: props.item.danger })} onClick={() => props.onDone()}>
            {props.item.label}
          </a>
        )}
      </Show>
    </Show>
  )
}

export function MenuItems(props: { items: MenuItem[]; onDone?: () => void; inline?: boolean }) {
  const done = () => props.onDone?.()
  return (
    <Show when={props.inline} fallback={<For each={props.items}>{(item) => <MenuItemButton item={item} onDone={done} kind="item" />}</For>}>
      <div class={menuItemsRowVariants()}>
        <For each={props.items}>{(item) => <MenuItemButton item={item} onDone={done} kind="button" />}</For>
      </div>
    </Show>
  )
}
