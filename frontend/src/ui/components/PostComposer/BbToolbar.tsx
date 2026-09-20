import { For, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../../i18n'
import type { MessageKey } from '../../i18n/ru'

const bbRowVariants = cva('flex flex-wrap items-center gap-1')
const bbBtnVariants = cva('cursor-pointer rounded-md bg-panel2 px-2.5 py-1 font-mono text-md text-fg hover:bg-hover')

interface BbTag {
  label: JSX.Element
  tag: string
  titleKey: MessageKey
}

const BB_TAGS: BbTag[] = [
  { label: <b>B</b>, tag: 'b', titleKey: 'postForm.bold' },
  { label: <i>I</i>, tag: 'i', titleKey: 'postForm.italic' },
  { label: <u>U</u>, tag: 'u', titleKey: 'postForm.underline' },
  { label: <span style={{ 'text-decoration': 'overline' }}>O</span>, tag: 'o', titleKey: 'postForm.overline' },
  { label: <s>S</s>, tag: 's', titleKey: 'postForm.strike' },
  { label: <sup>sup</sup>, tag: 'sup', titleKey: 'postForm.sup' },
  { label: <sub>sub</sub>, tag: 'sub', titleKey: 'postForm.sub' },
  { label: 'spoiler', tag: 'spoiler', titleKey: 'postForm.spoiler' },
  { label: 'url', tag: 'url', titleKey: 'postForm.url' },
  { label: 'code', tag: 'code', titleKey: 'postForm.code' },
  { label: 'secret', tag: 'secret', titleKey: 'postForm.secret' },
]

export function BbToolbar(props: { onInsertTag: (tag: string) => void }) {
  const { t } = useI18n()
  return (
    <div class={bbRowVariants()}>
      <For each={BB_TAGS}>
        {(btn) => (
          <button class={bbBtnVariants()} title={t(btn.titleKey)} type="button" onClick={() => props.onInsertTag(btn.tag)}>
            {btn.label}
          </button>
        )}
      </For>
    </div>
  )
}
