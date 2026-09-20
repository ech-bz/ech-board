import { createSignal } from 'solid-js'
import { cva } from 'class-variance-authority'
import { Modal } from './Modal'
import { Field } from './Field'
import { Button } from './Button'
import { useI18n } from '../i18n'

const topicFormVariants = cva('flex flex-col gap-4')
const topicActionsVariants = cva('flex justify-end gap-2')

export function ThreadTopicModal(props: {
  threadUid: string
  initial: string
  busy: boolean
  onSave: (threadUid: string, topic: string) => void
  onClose: () => void
}) {
  const { t } = useI18n()
  const [text, setText] = createSignal(props.initial)
  return (
    <Modal title={t('mod.editTopic')} onClose={props.onClose}>
      <div class={topicFormVariants()}>
        <Field as="input" grow="basis-full" value={text()} autofocus onInput={(e) => setText(e.currentTarget.value)} />
        <div class={topicActionsVariants()}>
          <Button disabled={props.busy} onClick={props.onClose}>
            {t('mod.cancel')}
          </Button>
          <Button disabled={props.busy} onClick={() => props.onSave(props.threadUid, text().trim())}>
            {t('mod.save')}
          </Button>
        </div>
      </div>
    </Modal>
  )
}
