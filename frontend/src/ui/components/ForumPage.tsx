import { cva } from 'class-variance-authority'
import { MenuItems, type MenuItem } from './Menu/MenuItems'
import { useI18n } from '../i18n'

const forumPageVariants = cva('mx-auto max-w-content px-6 py-12')
const forumDescVariants = cva('mb-4 text-lg leading-normal')
const forumStatsVariants = cva('mb-6 text-lg leading-normal')
const forumSelectVariants = cva('mt-6 text-lg leading-normal')

export function ForumPage(props: { boardsCount: number; totalPosts: number; menu: MenuItem[] }) {
  const { t } = useI18n()
  return (
    <div class={forumPageVariants()}>
      <p class={forumDescVariants()}>{t('ui.forumDescription')}</p>
      <p class={forumStatsVariants()}>{t('ui.forumStats', { boards: props.boardsCount, posts: props.totalPosts })}</p>
      <MenuItems items={props.menu} inline />
      <p class={forumSelectVariants()}>{t('ui.boardSelectText')}</p>
    </div>
  )
}
