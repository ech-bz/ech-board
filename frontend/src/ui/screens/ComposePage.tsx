import { cva } from 'class-variance-authority'
import { Show } from 'solid-js'
import { IconButton } from '../components/IconButton'
import { PageHeader } from '../components/PageHeader'
import { PostComposer } from '../components/PostComposer/PostComposer'
import { EllipsisVerticalIcon } from '../icons/EllipsisVerticalIcon'
import { useI18n } from '../i18n'
import { useIsMobile } from '../components/Gallery/useIsMobile'
import { useNav } from '../nav'

const viewSectionVariants = cva('flex min-h-screen min-w-0 flex-col')
const screenVariants = cva('flex min-h-dvh flex-col bg-bg')
const backBtnVariants = cva('px-1 py-2 hover:bg-hover')
const threadContentVariants = cva('pl-4')

export function ComposePage(props: { forumId: string; slug: string; heading: string }) {
  const { t } = useI18n()
  const nav = useNav()
  const mobile = useIsMobile()
  const back = () => nav.back()
  return (
    <Show
      when={mobile()}
      fallback={
        <section class={viewSectionVariants()}>
          <PageHeader title={t('ui.newThread')} subtitle={props.heading} onBack={back} />
          <div class={threadContentVariants()} />
          <PostComposer mode="thread" slug={props.slug} />
        </section>
      }
    >
      <section class={screenVariants()}>
        <PageHeader
          title={t('ui.newThread')}
          subtitle={props.heading}
          onBack={back}
          trailing={
            <IconButton size="md" variant="glass" class={backBtnVariants()}>
              <EllipsisVerticalIcon width={20} height={20} />
            </IconButton>
          }
        />
        <PostComposer mode="thread" slug={props.slug} />
      </section>
    </Show>
  )
}
