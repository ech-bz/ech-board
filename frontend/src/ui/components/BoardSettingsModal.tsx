import { createEffect, createSignal, For, Show } from 'solid-js'
import { cva } from 'class-variance-authority'
import { blake2b } from '@noble/hashes/blake2'
import { bcs } from '@mysten/bcs'
import { Address } from '../../core/bcs/common'
import { useI18n } from '../i18n'
import { BoardProjection, type BoardObject } from '../../core/bcs/types'
import { Modal } from './Modal'
import { Button } from './Button'
import { IconButton } from './IconButton'
import { Field } from './Field'
import { fromHex, toHex } from '../../core/intent/crypto'

const sectionVariants = cva('mb-4')
const labelVariants = cva('mb-2 font-bold')
const rowVariants = cva('flex flex-wrap items-center gap-2')
const fileLabelVariants = cva('inline-block cursor-pointer rounded-md bg-panel2 px-3 py-2 text-md text-fg')
const chipRowVariants = cva('mt-2.5 flex flex-wrap items-center gap-2.5')
const chipVariants = cva('relative size-12 overflow-hidden rounded-md bg-panel2')
const chipImgVariants = cva('size-full object-cover')
const chipRemoveVariants = cva('absolute right-0.5 top-0.5 bg-scrim text-sm leading-none')
const listVariants = cva('mt-2.5 flex flex-col gap-1.5')
const listItemVariants = cva('flex items-center gap-2 rounded-md bg-panel2 px-2.5 py-1.5 font-mono text-sm text-secondary break-all')
const listRemoveVariants = cva('cursor-pointer border-0 bg-transparent text-fg')
const toggleLabelVariants = cva('flex items-center gap-2 text-lg')
const hiddenInputVariants = cva('hidden')

const ReactionVec = bcs.vector(Address)

function parseAddress(s: string): Uint8Array | null {
  const b = fromHex(s)
  return b.length === 32 ? b : null
}

export interface BoardModActions {
  busy: boolean
  runBoardSetDescription: (text: string) => Promise<void>
  runBoardSetMaxMedia: (v: number) => Promise<void>
  runBoardSetBumpLimit: (v: number) => Promise<void>
  runBoardSetIgnoreForumBans: (v: boolean) => Promise<void>
  runBoardSetReactions: (vals: Uint8Array[]) => Promise<void>
  runBoardSetPinned: (pinned: Uint8Array[]) => Promise<void>
  runBoardAddModerator: (addr: Uint8Array) => Promise<void>
  runBoardDelModerator: (addr: Uint8Array) => Promise<void>
}

export function BoardSettingsModal(props: {
  onClose: () => void
  board: BoardObject
  description: string
  moderators: Uint8Array[]
  relayUrl: string
  boardUid: string
  canCore: boolean
  canSoft: boolean
  mod: BoardModActions
}) {
  const { t } = useI18n()
  const [desc, setDesc] = createSignal('')
  const [maxMedia, setMaxMedia] = createSignal('')
  const [bumpLimit, setBumpLimit] = createSignal('')
  const [ignoreBans, setIgnoreBans] = createSignal(false)
  const [reactions, setReactions] = createSignal<Uint8Array[]>([])
  const [pinned, setPinned] = createSignal<Uint8Array[]>([])
  const [newPin, setNewPin] = createSignal('')
  const [newMod, setNewMod] = createSignal('')

  createEffect(() => {
    setDesc(props.description)
    const bp = new BoardProjection(props.board.projection)
    setMaxMedia(String(bp.max_media()))
    setBumpLimit(String(bp.bump_limit()))
    setIgnoreBans(!!bp.ignore_forum_bans())
    setReactions(((bp.reactions() as Uint8Array[] | undefined) ?? []).map((entry) => new Uint8Array(entry)))
    setPinned(((bp.pinned() as Uint8Array[] | undefined) ?? []).map((entry) => new Uint8Array(entry)))
    setNewPin('')
    setNewMod('')
  })

  const saveNumber = (fn: (v: number) => void) => (raw: string) => {
    const n = Number.parseInt(raw, 10)
    if (!Number.isNaN(n)) void fn(n)
  }

  const onReactionFile = async (input: HTMLInputElement) => {
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    const bytes = new Uint8Array(await file.arrayBuffer())
    const hash = blake2b(bytes, { dkLen: 32 })
    const hex = toHex(hash)
    if (reactions().some((entry) => toHex(entry) === hex)) return
    const resp = await fetch(`${props.relayUrl}/content/${props.boardUid}/reaction/${hex}`, { method: 'PUT', body: bytes })
    if (!resp.ok) return
    setReactions([...reactions(), hash])
  }

  const copyReactions = async () => {
    if (reactions().length === 0) return
    const bytes = ReactionVec.serialize(reactions()).toBytes()
    try {
      await navigator.clipboard.writeText(toHex(bytes))
    } catch {
      return
    }
  }

  const pasteReactions = async () => {
    const text = (await navigator.clipboard.readText()).trim()
    if (!text) return
    let parsed: Uint8Array[]
    try {
      parsed = ReactionVec.parse(fromHex(text)) as Uint8Array[]
    } catch {
      return
    }
    const next = [...reactions()]
    for (const entry of parsed) {
      const hex = toHex(entry)
      if (!next.some((item) => toHex(item) === hex)) next.push(entry)
    }
    setReactions(next)
  }

  const addPin = () => {
    const address = parseAddress(newPin())
    if (!address || pinned().some((entry) => toHex(entry) === toHex(address))) return
    setPinned([...pinned(), address])
    setNewPin('')
  }

  const addModerator = () => {
    const address = parseAddress(newMod())
    if (!address) return
    void props.mod.runBoardAddModerator(address)
    setNewMod('')
  }

  return (
    <Modal title={t('settings.board.title')} onClose={props.onClose}>
      <Show when={props.canSoft}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('settings.board.description')}</div>
          <div class={rowVariants()}>
            <Field as="input" grow="min-w-input" value={desc()} onInput={(e) => setDesc(e.currentTarget.value)} />
            <Button disabled={props.mod.busy} onClick={() => void props.mod.runBoardSetDescription(desc())}>
              {t('mod.save')}
            </Button>
          </div>
        </div>
      </Show>

      <Show when={props.canCore}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('settings.board.maxMedia')}</div>
          <div class={rowVariants()}>
            <Field as="input" type="number" grow="min-w-input" value={maxMedia()} onInput={(e) => setMaxMedia(e.currentTarget.value)} />
            <Button disabled={props.mod.busy} onClick={() => saveNumber(props.mod.runBoardSetMaxMedia)(maxMedia())}>
              {t('mod.save')}
            </Button>
          </div>
        </div>
      </Show>

      <Show when={props.canCore}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('settings.board.bumpLimit')}</div>
          <div class={rowVariants()}>
            <Field as="input" type="number" grow="min-w-input" value={bumpLimit()} onInput={(e) => setBumpLimit(e.currentTarget.value)} />
            <Button disabled={props.mod.busy} onClick={() => saveNumber(props.mod.runBoardSetBumpLimit)(bumpLimit())}>
              {t('mod.save')}
            </Button>
          </div>
        </div>
      </Show>

      <Show when={props.canCore}>
        <div class={sectionVariants()}>
          <label class={toggleLabelVariants()}>
            <input
              type="checkbox"
              checked={ignoreBans()}
              onChange={(e) => {
                setIgnoreBans(e.currentTarget.checked)
                void props.mod.runBoardSetIgnoreForumBans(e.currentTarget.checked)
              }}
            />
            <span>{t('settings.board.ignoreForumBans')}</span>
          </label>
        </div>
      </Show>

      <Show when={props.canSoft}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('settings.board.reactions')}</div>
          <div class={rowVariants()}>
            <label>
              <input
                type="file"
                class={hiddenInputVariants()}
                accept="image/*"
                disabled={props.mod.busy}
                onChange={(e) => void onReactionFile(e.currentTarget)}
              />
              <span class={fileLabelVariants()}>{t('settings.board.addReaction')}</span>
            </label>
            <Button disabled={props.mod.busy || reactions().length === 0} onClick={() => void copyReactions()}>
              {t('settings.board.copyReactions')}
            </Button>
            <Button disabled={props.mod.busy} onClick={() => void pasteReactions()}>
              {t('settings.board.pasteReactions')}
            </Button>
            <Button disabled={props.mod.busy || reactions().length === 0} onClick={() => void props.mod.runBoardSetReactions(reactions())}>
              {t('mod.save')}
            </Button>
          </div>
          <div class={chipRowVariants()}>
            <For each={reactions()}>
              {(reaction, index) => (
                <div class={chipVariants()}>
                  <img class={chipImgVariants()} src={`${props.relayUrl}/content/${props.boardUid}/reaction/${toHex(reaction)}`} alt="" />
                  <IconButton
                    size="xs"
                    class={chipRemoveVariants()}
                    disabled={props.mod.busy}
                    onClick={() => setReactions(reactions().filter((_, i) => i !== index()))}
                  >
                    x
                  </IconButton>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>

      <Show when={props.canSoft}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('settings.board.pinnedThreads')}</div>
          <div class={rowVariants()}>
            <Field
              as="input"
              grow="min-w-input"
              value={newPin()}
              placeholder={t('settings.board.pinnedPlaceholder')}
              onInput={(e) => setNewPin(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') addPin()
              }}
            />
            <Button disabled={props.mod.busy} onClick={addPin}>
              {t('settings.board.addPinned')}
            </Button>
            <Button disabled={props.mod.busy || pinned().length === 0} onClick={() => void props.mod.runBoardSetPinned(pinned())}>
              {t('mod.save')}
            </Button>
          </div>
          <div class={listVariants()}>
            <For each={pinned()}>
              {(entry, index) => (
                <div class={listItemVariants()}>
                  <span>{toHex(entry)}</span>
                  <button
                    type="button"
                    class={listRemoveVariants()}
                    disabled={props.mod.busy}
                    onClick={() => setPinned(pinned().filter((_, i) => i !== index()))}
                  >
                    x
                  </button>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>

      <Show when={props.canCore}>
        <div class={sectionVariants()}>
          <div class={labelVariants()}>{t('mod.moderators')}</div>
          <div class={rowVariants()}>
            <Field
              as="input"
              grow="min-w-input"
              value={newMod()}
              placeholder={t('settings.addrPlaceholder')}
              onInput={(e) => setNewMod(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') addModerator()
              }}
            />
            <Button disabled={props.mod.busy || !parseAddress(newMod())} onClick={addModerator}>
              {t('settings.add')}
            </Button>
          </div>
          <div class={listVariants()}>
            <For each={props.moderators}>
              {(moderator) => (
                <div class={listItemVariants()}>
                  <span>{toHex(moderator)}</span>
                  <button type="button" class={listRemoveVariants()} disabled={props.mod.busy} onClick={() => void props.mod.runBoardDelModerator(moderator)}>
                    {t('settings.remove')}
                  </button>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>
    </Modal>
  )
}
