import { createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../i18n'
import type { MessageKey } from '../i18n/ru'
import { Modal } from './Modal'
import { Button } from './Button'
import { Field } from './Field'
import { canBan, type BanScope } from '../../core/intent/capabilities'
import type { RoleKind } from '../../core/intent/roles'

const sectionVariants = cva('mb-4')
const labelVariants = cva('mb-2 font-bold')
const rowVariants = cva('flex flex-wrap items-center gap-2')

const MASKS = [32, 24, 20, 16]

const DURATIONS: { key: MessageKey; ms: number }[] = [
  { key: 'ban.dur1h', ms: 3600_000 },
  { key: 'ban.dur1d', ms: 86400_000 },
  { key: 'ban.dur7d', ms: 7 * 86400_000 },
  { key: 'ban.dur30d', ms: 30 * 86400_000 },
  { key: 'ban.dur365d', ms: 365 * 86400_000 },
  { key: 'ban.durForever', ms: Number.MAX_SAFE_INTEGER },
]

const SCOPE_LABELS: Record<BanScope, MessageKey> = {
  thread: 'ban.scopeThread',
  board: 'ban.scopeBoard',
  forum: 'ban.scopeForum',
}

function isUidHex(value: string): boolean {
  const hex = value.trim().replace(/^0x/, '')
  return hex.length > 0 && hex.length % 2 === 0 && /^[0-9a-fA-F]+$/.test(hex)
}

export function BanModal(props: {
  mode?: 'post' | 'uid'
  scopes?: BanScope[]
  roleKinds: RoleKind[]
  busy?: boolean
  onClose: () => void
  onBan: (scope: BanScope, mask: number, durationMs: number, reason: string, uidHex?: string) => void
}) {
  const { t } = useI18n()
  const [scope, setScope] = createSignal<BanScope>('thread')
  const [mask, setMask] = createSignal(32)
  const [durationIdx, setDurationIdx] = createSignal(2)
  const [reason, setReason] = createSignal('')
  const [uid, setUid] = createSignal('')
  const uidMode = () => props.mode === 'uid'
  const scopes = () =>
    (['thread', 'board', 'forum'] as BanScope[]).filter(
      (entry) => canBan(entry, props.roleKinds) && (!props.scopes || props.scopes.includes(entry)),
    )
  const effectiveScope = () => (scopes().includes(scope()) ? scope() : (scopes()[0] ?? 'thread'))
  const canSubmit = () => scopes().length > 0 && (!uidMode() || isUidHex(uid()))
  return (
    <Modal title={t(uidMode() ? 'ban.titleUid' : 'ban.title')} onClose={props.onClose}>
      <Show when={uidMode()}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('ban.uid')}</div>
          <div class={rowVariants()}>
            <Field as="input" grow="min-w-input" value={uid()} onInput={(e) => setUid(e.currentTarget.value)} />
          </div>
        </div>
      </Show>
      <div class={sectionVariants()}>
        <div class={labelVariants()}>{t('ban.scope')}</div>
        <Field as="select" value={effectiveScope()} onInput={(e) => setScope(e.currentTarget.value as BanScope)}>
          <For each={scopes()}>{(entry) => <option value={entry}>{t(SCOPE_LABELS[entry])}</option>}</For>
        </Field>
      </div>
      <div class={sectionVariants()}>
        <div class={labelVariants()}>{t('ban.mask')}</div>
        <Field as="select" value={String(mask())} onInput={(e) => setMask(Number.parseInt(e.currentTarget.value, 10))}>
          <For each={MASKS}>{(entry) => <option value={String(entry)}>/{entry}</option>}</For>
        </Field>
      </div>
      <div class={sectionVariants()}>
        <div class={labelVariants()}>{t('ban.duration')}</div>
        <Field as="select" value={String(durationIdx())} onInput={(e) => setDurationIdx(Number.parseInt(e.currentTarget.value, 10))}>
          <For each={DURATIONS}>
            {(entry, index) => <option value={String(index())}>{t(entry.key)}</option>}
          </For>
        </Field>
      </div>
      <div class={sectionVariants()}>
        <div class={labelVariants()}>{t('ban.reason')}</div>
        <div class={rowVariants()}>
          <Field as="input" grow="min-w-input" value={reason()} onInput={(e) => setReason(e.currentTarget.value)} />
          <Button
            disabled={props.busy || !canSubmit()}
            onClick={() =>
              props.onBan(effectiveScope(), mask(), DURATIONS[durationIdx()].ms, reason(), uidMode() ? uid().trim() : undefined)
            }
          >
            {t('mod.save')}
          </Button>
        </div>
      </div>
    </Modal>
  )
}
