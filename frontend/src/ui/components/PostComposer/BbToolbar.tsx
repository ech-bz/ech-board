import { For, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { IconButton } from '../IconButton'
import { useI18n } from '../../i18n'
import type { MessageKey } from '../../i18n/ru'
import { BoldIcon } from '../../icons/BoldIcon'
import { ItalicIcon } from '../../icons/ItalicIcon'
import { StrikethroughIcon } from '../../icons/StrikethroughIcon'
import { EyeOffIcon } from '../../icons/EyeOffIcon'
import { CodeIcon } from '../../icons/CodeIcon'
import { UnderlineIcon } from '../../icons/UnderlineIcon'
import { OverlineIcon } from '../../icons/OverlineIcon'
import { SuperscriptIcon } from '../../icons/SuperscriptIcon'
import { SubscriptIcon } from '../../icons/SubscriptIcon'
import { FoldIcon } from '../../icons/FoldIcon'
import { SecretIcon } from '../../icons/SecretIcon'

const rowVariants = cva(
  'flex w-full flex-wrap items-center gap-1.5 min-[900px]:w-auto min-[900px]:gap-2.5',
)
const ICONS: Record<string, () => JSX.Element> = {
  b: () => <BoldIcon width={24} height={24} />,
  i: () => <ItalicIcon width={24} height={24} />,
  u: () => <UnderlineIcon width={24} height={24} />,
  o: () => <OverlineIcon width={24} height={24} />,
  s: () => <StrikethroughIcon width={24} height={24} />,
  sup: () => <SuperscriptIcon width={24} height={24} />,
  sub: () => <SubscriptIcon width={24} height={24} />,
  spoiler: () => <EyeOffIcon width={24} height={24} />,
  fold: () => <FoldIcon width={24} height={24} />,
  code: () => <CodeIcon width={24} height={24} />,
  secret: () => <SecretIcon width={24} height={24} />,
}

const TEXT_LABEL: Record<string, string> = {
  url: 'url',
}

interface BbTag {
  tag: string
  titleKey: MessageKey
}

const BB_TAGS: BbTag[] = [
  { tag: 'b', titleKey: 'postForm.bold' },
  { tag: 'i', titleKey: 'postForm.italic' },
  { tag: 'u', titleKey: 'postForm.underline' },
  { tag: 'o', titleKey: 'postForm.overline' },
  { tag: 's', titleKey: 'postForm.strike' },
  { tag: 'sup', titleKey: 'postForm.sup' },
  { tag: 'sub', titleKey: 'postForm.sub' },
  { tag: 'spoiler', titleKey: 'postForm.spoiler' },
  { tag: 'fold', titleKey: 'postForm.fold' },
  { tag: 'url', titleKey: 'postForm.url' },
  { tag: 'code', titleKey: 'postForm.code' },
  { tag: 'secret', titleKey: 'postForm.secret' },
]

export function BbToolbar(props: { onInsertTag: (tag: string) => void }) {
  const { t } = useI18n()
  return (
    <div class={rowVariants()}>
      <For each={BB_TAGS}>
        {(btn) => (
          <IconButton
            size="md"
            title={t(btn.titleKey)}
            aria-label={t(btn.titleKey)}
            onClick={() => props.onInsertTag(btn.tag)}
          >
            {btn.tag in ICONS ? (
              ICONS[btn.tag]()
            ) : (
              <span class="grid size-6 place-items-center font-mono text-[15px] font-bold leading-none">{TEXT_LABEL[btn.tag] ?? btn.tag}</span>
            )}
          </IconButton>
        )}
      </For>
    </div>
  )
}
