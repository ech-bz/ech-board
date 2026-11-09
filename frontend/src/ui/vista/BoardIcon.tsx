import { vistaBoardIcon } from './boardIcons'

export function BoardIcon(props: { slug: string; title?: string; size?: number; class?: string }) {
  const size = () => props.size ?? 32
  return (
    <img
      class={props.class ?? 'vista-board-icon'}
      src={vistaBoardIcon(props.slug, props.title)}
      width={size()}
      height={size()}
      alt=""
      draggable={false}
    />
  )
}
