import { createEffect, createMemo, createSignal, onCleanup, Show, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { useI18n } from '../../i18n'
import { ClipIcon } from '../../icons/ClipIcon'
import { SendIcon } from '../../icons/SendIcon'
import { XIcon } from '../../icons/XIcon'
import { ChevronUpIcon } from '../../icons/ChevronUpIcon'
import { ChevronDownIcon } from '../../icons/ChevronDownIcon'
import { createPostSubmit } from '../../postSubmit'
import { detectRoles, matchesAuthor, type RoleKind } from '../../../core/intent/roles'
import type { PostPartInput } from '../../../core/bcs/types'
import { partsToEditText, postParts } from '../../../core/posts'
import { useEntityActions } from '../../entityActions'
import { getSecretKeyBytes } from '../../../core/keys/storage'
import { fromHex } from '../../../core/intent/crypto'
import { parseMoveAbort } from '../../../core/intent/errors'
import { getForumInfo } from '../../../core/api/cache'
import { IconButton } from '../IconButton'
import { useComposerBridge } from '../../composerBridge'
import { getComposerDraft, saveComposerDraft } from '../../composerDraft'
import { reportError } from '../../errors'
import { useRecordSource, useStore } from '../../store'
import { OptionsPanel } from './OptionsPanel'
import { BbToolbar } from './BbToolbar'
import { FilePreview } from './FilePreview'
import { createOptionsReveal } from './useOptionsReveal'
import { closeMobileComposer, mobileComposerOpen, openMobileComposer } from '../../composerVisibility'

const spinnerVariants = cva('size-4 animate-spin rounded-full border-2 border-track border-t-fg')
const composerRootVariants = cva('composer-fixed layout-anim')
const boxVariants = cva(
  'relative flex flex-col gap-1.5 rounded-[var(--radius-composer)] bg-panel1 p-2 glass',
)
const dropOverlayVariants = cva(
  'pointer-events-none absolute inset-0 z-10 flex items-center justify-center rounded-lg border-2 border-dashed border-accent bg-bg text-accent uppercase [letter-spacing:0.03em]',
)
const actionRowVariants = cva('flex items-end gap-2.5')
const textareaVariants = cva(
  'box-border min-h-[44px] max-h-composer w-full min-w-0 flex-1 resize-none overflow-y-hidden bg-transparent pt-[10px] pb-0 text-left text-[15px] font-normal leading-6 text-fg outline-none',
)
const mirrorVariants = cva('box-border min-h-[44px] max-h-composer resize-none overflow-hidden pt-[10px] pb-0 text-[15px] font-normal leading-6')
const hiddenInputVariants = cva('hidden')
const toolVariants = cva('grid size-11 flex-none place-items-center rounded-full bg-chip text-secondary transition-colors hover:bg-hover hover:text-fg')
const sendVariants = cva(
  'ml-1.5 grid size-11 flex-none place-items-center rounded-full bg-accent text-[#0b1626] transition-colors hover:brightness-110 disabled:bg-chip disabled:text-secondary',
)
const filesOverlayVariants = cva('mb-1.5 flex justify-start px-0.5')
const optionsWrapVariants = cva('min-w-0', {
  variants: {
    open: {
      true: 'block',
      false: 'hidden',
    },
  },
})
const optionsInnerVariants = cva('min-w-0')
const optionsStackVariants = cva('flex flex-col gap-1.5 pb-0.5 pt-2')

function isAllowedMedia(type: string): boolean {
  return (
    type.startsWith('image/') ||
    type === 'video/mp4' ||
    type === 'video/webm' ||
    type === 'audio/mpeg' ||
    type === 'audio/ogg' ||
    type === 'application/pdf'
  )
}

export function PostComposer(props: { mode?: 'reply' | 'thread'; slug?: string; threadNum?: number; editingUid?: string | null; onEditDone?: () => void; nav?: JSX.Element }) {
  const { t } = useI18n()
  const store = useStore()
  const ea = useEntityActions()
  const { setInsert } = useComposerBridge()
  const mode = () => props.mode ?? 'reply'
  const isThread = () => mode() === 'thread'
  const editing = useRecordSource(() => props.editingUid ?? '')
  const isEdit = () => !!props.editingUid

  const draft = getComposerDraft(mode())
  const [text, setText] = createSignal(draft.text)
  const [subject, setSubject] = createSignal(draft.subject)
  const [name, setName] = createSignal(draft.name)
  const [sendAsKind, setSendAsKind] = createSignal<RoleKind>(draft.sendAsKind)
  const [files, setFiles] = createSignal<File[]>(draft.files)
  const [maxMedia, setMaxMedia] = createSignal<number | null>(null)
  const [secretRefs, setSecretRefs] = createSignal<Map<number, PostPartInput> | null>(null)
  const [withFlag, setWithFlag] = createSignal(draft.withFlag)
  const [stripExifOn, setStripExifOn] = createSignal(draft.stripExifOn)
  const [selfAdmin, setSelfAdmin] = createSignal(draft.selfAdmin)
  const [dragOver, setDragOver] = createSignal(false)
  const [optionsOpen, setOptionsOpen] = createSignal(false)
  const reveal = createOptionsReveal(() => optionsOpen() || isEdit())

  createEffect(() => {
    const open = mobileComposerOpen() || isEdit() || mode() === 'reply'
    document.documentElement.classList.toggle('composer-open', open)
    onCleanup(() => document.documentElement.classList.remove('composer-open'))
  })

  createEffect(() => {
    if (!mobileComposerOpen() && !isEdit()) return
    const onPointer = (e: PointerEvent) => {
      if (window.innerWidth > 899) return
      const target = e.target as Node | null
      if (!target) return
      if (root?.contains(target)) return
      if ((target as HTMLElement).closest?.('[data-tabs]')) return
      if (mode() === 'reply' || isEdit()) return
      closeMobileComposer()
      setOptionsOpen(false)
    }
    document.addEventListener('pointerdown', onPointer, true)
    onCleanup(() => document.removeEventListener('pointerdown', onPointer, true))
  })

  const submitter = createPostSubmit({
    onError: (e) => reportError(parseMoveAbort(e)?.message ?? String(e)),
  })

  const token = {}
  let textarea: HTMLTextAreaElement | undefined
  let mirror: HTMLTextAreaElement | undefined
  let fileInput: HTMLInputElement | undefined
  let root: HTMLDivElement | undefined
  let pendingCaret: number | null = null
  let capped = false

  const roles = createMemo(() => {
    const info = store.info()
    if (!info) return []
    const master = getSecretKeyBytes()
    const forumIdBytes = fromHex(info.forumId)
    const opUid = store.opUid()
    const opRecord = opUid ? store.get(opUid) : undefined
    const op =
      opRecord && matchesAuthor(master, forumIdBytes, opRecord.senderPk, opRecord.senderTweak)
        ? { tweak: opRecord.senderTweak, pk: opRecord.senderPk }
        : null
    return detectRoles({
      master,
      forumIdBytes,
      forumIdHex: info.forumId,
      boardUidHex: info.boardUid,
      threadUidHex: store.threadUid() ?? undefined,
      moderators: info.moderators,
      op,
    })
  })

  const visibleRoles = () => roles().filter((item) => item.selectable !== false)

  createEffect(() => {
    if (!visibleRoles().some((item) => item.kind === sendAsKind())) setSendAsKind('anonymous')
  })

  const canSend = () =>
    isEdit()
      ? !!props.editingUid && !ea.busy() && text().trim().length > 0
      : !submitter.creating() && (files().length > 0 || text().trim().length > 0)

  const addFiles = (incoming: FileList | File[]) => {
    const arr = Array.from(incoming).filter((file) => isAllowedMedia(file.type))
    if (arr.length === 0) return
    let accepted = arr
    const limit = maxMedia()
    if (limit != null && limit >= 0) {
      const room = Math.max(0, limit - files().length)
      if (room < arr.length) {
        if (room === 0) {
          reportError(t('postForm.maxMediaReached', { n: limit }))
          return
        }
        reportError(t('postForm.maxMediaPartial', { n: limit, added: room, skipped: arr.length - room }))
        accepted = arr.slice(0, room)
      }
    }
    setFiles((prev) => [...prev, ...accepted])
  }

  const removeFile = (idx: number) => setFiles((prev) => prev.filter((_, i) => i !== idx))

  const previewUrls = createMemo(() => new Map(files().map((file) => [file, URL.createObjectURL(file)] as const)))

  createEffect(() => {
    const urls = previewUrls()
    return () => {
      for (const url of urls.values()) URL.revokeObjectURL(url)
    }
  })

  createEffect(() => {
    saveComposerDraft(mode(), {
      text: text(),
      subject: subject(),
      name: name(),
      sendAsKind: sendAsKind(),
      optionsOpen: optionsOpen(),
      files: files(),
      withFlag: withFlag(),
      stripExifOn: stripExifOn(),
      selfAdmin: selfAdmin(),
    })
  })

  createEffect(() => {
    const info = store.info()
    const slug = props.slug
    if (!info || !slug) return
    let cancelled = false
    getForumInfo(info.relayUrl)
      .then((forum) => {
        if (!cancelled) setMaxMedia(forum.boardMaxMedia.get(slug) ?? null)
      })
      .catch((e) => reportError(e))
    return () => {
      cancelled = true
    }
  })

  const keepScrollTop = () => {
    const el = textarea
    if (el && !capped) el.scrollTop = 0
  }
  const followEnd = () => {
    const el = textarea
    if (!el) return
    if (!capped) {
      el.scrollTop = 0
      return
    }
    if (el.selectionEnd === el.value.length) el.scrollTop = el.scrollHeight
  }
  const BASE_H = 44 
  const syncHeight = () => {
    const value = text()
    const el = textarea
    const box = mirror
    if (!el || !box) return
    const wasCapped = capped
    box.style.width = `${el.clientWidth}px`
    box.value = value || ' '
    box.style.height = 'auto'
    const height = Math.max(BASE_H, box.scrollHeight)
    el.style.height = `${height}px`
    const cap = Number.parseFloat(getComputedStyle(box).maxHeight)
    capped = height > cap + 1
    el.style.overflowY = capped ? 'auto' : 'hidden'
    if (!capped) el.scrollTop = 0
    if (capped !== wasCapped) syncHeight()
  }
  createEffect(() => {
    syncHeight()
    followEnd()
  })
  window.addEventListener('resize', syncHeight)
  onCleanup(() => window.removeEventListener('resize', syncHeight))

  createEffect(() => {
    const target = editing()
    if (!target) {
      setSecretRefs(null)
      return
    }
    const content = store.content()
    if (!content) return
    const { text: src, secrets } = partsToEditText(postParts(target.post, content))
    setText(src)
    setSecretRefs(secrets)
    setOptionsOpen(true)
    openMobileComposer()
  })

  const insertTag = (tag: string) => {
    const el = textarea
    if (!el) return
    const start = el.selectionStart
    const end = el.selectionEnd
    const before = text().slice(0, start)
    const selected = text().slice(start, end)
    const after = text().slice(end)
    let next: string
    if (tag === 'url') {
      next = before + `[url=]${selected}[/url]` + after
    } else if (tag === 'fold') {
      next = before + `[fold=]${selected}[/fold]` + after
    } else if (selected) {
      next = before + `[${tag}]${selected}[/${tag}]` + after
    } else {
      next = before + `[${tag}][/${tag}]` + after
    }
    setText(next)
    el.focus()
  }

  const insertReply = (quote: string) => {
    const el = textarea
    const ins = `${quote}\n`
    const start = el ? el.selectionStart : 0
    const end = el ? el.selectionEnd : 0
    pendingCaret = start + ins.length
    setText((prev) => `${prev.slice(0, start)}${ins}${prev.slice(end)}`)
  }

  createEffect(() => {
    const el = textarea
    const pos = pendingCaret
    text()
    if (!el || pos == null) return
    pendingCaret = null
    el.focus()
    el.setSelectionRange(pos, pos)
    followEnd()
  })

  createEffect(() => {
    setInsert(insertReply, token, () => !!root && root.getClientRects().length > 0)
  })

  const submitReply = async () => {
    const info = store.info()
    const threadUid = store.threadUid()
    if (!info || !threadUid) return
    const role = roles().find((item) => item.kind === sendAsKind())
    const res = await submitter.createReply(
      {
        forumId: info.forumId,
        nonceShardsId: info.nonceShardsId,
        relayUrl: info.relayUrl,
        boardUid: info.boardUid,
        slug: props.slug,
        threadUid,
        postMap: store.postMap(),
        role,
      },
      { text: text(), name: name(), withFlag: withFlag(), files: files(), stripExif: stripExifOn(), selfAdmin: false },
    )
    if (!res) return
    setText('')
    setName('')
    setFiles([])
    setWithFlag(false)
    setOptionsOpen(false)
  }

  const createThread = async () => {
    const info = store.info()
    const slug = props.slug
    if (!info || !slug) return
    const role = roles().find((item) => item.kind === sendAsKind())
    const res = await submitter.createThread(
      { forumId: info.forumId, nonceShardsId: info.nonceShardsId, relayUrl: info.relayUrl, boardUid: info.boardUid, slug, role },
      { text: text(), name: name(), withFlag: withFlag(), files: files(), stripExif: stripExifOn(), selfAdmin: selfAdmin() },
      subject(),
    )
    if (!res) return
    setText('')
    setSubject('')
    setName('')
    setFiles([])
    setWithFlag(false)
    setSelfAdmin(false)
    setOptionsOpen(false)
  }

  const submitEdit = async () => {
    const target = editing()
    const threadUid = store.threadUid()
    if (!target || !threadUid) return
    await ea.runPostEdit(target.post, threadUid, text(), store.postMap(), secretRefs() ?? new Map())
    setText('')
    setSecretRefs(null)
    props.onEditDone?.()
  }

  const submit = () => {
    if (isEdit()) void submitEdit()
    else if (isThread()) void createThread()
    else void submitReply()
  }

  return (
    <div
      ref={root}
      data-composer=""
      class={composerRootVariants()}
      onDragEnter={(e) => {
        e.preventDefault()
        setDragOver(true)
      }}
      onDragOver={(e) => e.preventDefault()}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node)) setDragOver(false)
      }}
      onDrop={(e) => {
        e.preventDefault()
        setDragOver(false)
        addFiles(e.dataTransfer?.files ?? [])
      }}
      onPaste={(e) => {
        const items = e.clipboardData?.items
        if (!items) return
        const media: File[] = []
        for (let i = 0; i < items.length; i++) {
          const item = items[i]
          if (isAllowedMedia(item.type)) {
            const file = item.getAsFile()
            if (file) media.push(file)
          }
        }
        if (media.length > 0) {
          e.preventDefault()
          addFiles(media)
        }
      }}
    >
      <div class="relative w-full">
        <div ref={reveal.setAbove} class="absolute inset-x-0 bottom-full">
          {props.nav}
          <Show when={files().length > 0}>
            <div class={filesOverlayVariants()}>
              <FilePreview files={files()} previewUrls={previewUrls()} onRemove={removeFile} />
            </div>
          </Show>
        </div>
      <div ref={reveal.setBox} class={boxVariants()} style={{ position: 'relative' }}>
        <div class="pointer-events-none invisible absolute left-0 top-0 h-0 w-0 overflow-hidden">
          <textarea ref={mirror} class={mirrorVariants()} rows={1} tabIndex={-1} aria-hidden="true" />
        </div>
        <Show when={dragOver()}>
          <div class={dropOverlayVariants()}>{t('postForm.dropHere')}</div>
        </Show>

        <input
          ref={fileInput}
          type="file"
          accept="image/*,video/mp4,video/webm,audio/mpeg,audio/ogg,application/pdf"
          multiple
          class={hiddenInputVariants()}
          onChange={(e) => {
            const selected = e.currentTarget.files
            if (selected) {
              addFiles(selected)
              openMobileComposer()
            }
          }}
        />

        <div ref={reveal.setOptions} class={optionsWrapVariants({ open: reveal.shown() })}>
          <div class={optionsInnerVariants()}>
            <div class={optionsStackVariants()}>
              <Show when={isThread() && !isEdit()}>
                <input
                  class="h-9 w-full min-w-0 rounded-full border-0 bg-panel2 px-3 text-left text-[15px] text-fg outline-none placeholder:text-secondary"
                  value={subject()}
                  onInput={(e) => setSubject(e.currentTarget.value)}
                  placeholder={t('postForm.subject')}
                />
              </Show>

              <div class="flex w-full min-w-0 flex-col gap-1.5 min-[900px]:flex-row min-[900px]:items-center min-[900px]:gap-2">
                <Show when={isEdit()}>
                  <IconButton size="md" class={toolVariants()} title={t('mod.cancel')} onClick={() => props.onEditDone?.()}>
                    <XIcon width={24} height={24} />
                  </IconButton>
                </Show>
                <div class="order-2 min-w-0 min-[900px]:order-1 min-[900px]:min-w-0 min-[900px]:flex-1">
                  <BbToolbar onInsertTag={insertTag} />
                </div>
                <Show when={!isEdit()}>
                  <div class="order-1 w-full min-[900px]:order-2 min-[900px]:w-auto min-[900px]:flex-none">
                    <OptionsPanel
                      variant="meta"
                      isEdit={false}
                      isThread={isThread()}
                      name={name()}
                      onNameChange={setName}
                      sendAsKind={sendAsKind()}
                      onSendAsKindChange={setSendAsKind}
                      roles={visibleRoles()}
                      selfAdmin={selfAdmin()}
                      onSelfAdminChange={setSelfAdmin}
                      subject={subject()}
                      onSubjectChange={setSubject}
                      withFlag={withFlag()}
                      onWithFlagChange={setWithFlag}
                      stripExifOn={stripExifOn()}
                      onStripExifChange={setStripExifOn}
                    />
                  </div>
                </Show>
              </div>

              <Show when={!isEdit()}>
                <OptionsPanel
                  variant="checks"
                  isEdit={false}
                  isThread={isThread()}
                  name={name()}
                  onNameChange={setName}
                  sendAsKind={sendAsKind()}
                  onSendAsKindChange={setSendAsKind}
                  roles={visibleRoles()}
                  selfAdmin={selfAdmin()}
                  onSelfAdminChange={setSelfAdmin}
                  subject={subject()}
                  onSubjectChange={setSubject}
                  withFlag={withFlag()}
                  onWithFlagChange={setWithFlag}
                  stripExifOn={stripExifOn()}
                  onStripExifChange={setStripExifOn}
                />
              </Show>
            </div>
          </div>
        </div>

        <div class={actionRowVariants()}>
          <Show when={!isEdit()}>
            <IconButton
              size="md"
              class={toolVariants()}
              title={optionsOpen() ? t('postForm.collapse') : t('postForm.expand')}
              aria-label={optionsOpen() ? t('postForm.collapse') : t('postForm.expand')}
              onClick={() => setOptionsOpen(!optionsOpen())}
            >
              <Show when={optionsOpen()} fallback={<ChevronUpIcon width={24} height={24} />}>
                <ChevronDownIcon width={24} height={24} />
              </Show>
            </IconButton>
            <IconButton size="md" class={toolVariants()} title={t('postForm.addFile')} onClick={() => fileInput?.click()}>
              <ClipIcon width={24} height={24} />
            </IconButton>
          </Show>
          <textarea
            ref={textarea}
            class={textareaVariants()}
            placeholder={
              isThread()
                ? t('postForm.newThread')
                : isEdit()
                  ? t('postForm.comment')
                  : t('postForm.newMessage')
            }
            rows={1}
            value={text()}
            onScroll={keepScrollTop}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                e.preventDefault()
                if (canSend()) submit()
              }
            }}
          />
          <IconButton
            size="md"
            class={sendVariants()}
            disabled={!canSend()}
            onClick={submit}
            title={isEdit() ? t('mod.save') : t('postForm.submit')}
          >
            <Show when={isEdit() ? ea.busy() : submitter.creating()} fallback={<SendIcon width={24} height={24} />}>
              <span class={spinnerVariants()} />
            </Show>
          </IconButton>
        </div>
      </div>
      </div>
    </div>
  )
}
