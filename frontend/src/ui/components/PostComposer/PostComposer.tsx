import { createEffect, createMemo, createSignal, Show, type JSX } from 'solid-js'
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
import { FilePreview } from './FilePreview'

const spinnerVariants = cva('size-4 animate-spin rounded-full border-2 border-track border-t-fg')
const composerRootVariants = cva(
  'glass-fade glass-fade--up sticky bottom-0 left-0 right-0 z-nav mt-auto flex flex-col gap-1.5 px-4 pb-[calc(10px+env(safe-area-inset-bottom,0px))] pt-2.5 min-[900px]:min-h-[var(--global-bar-h)]',
)
const dropOverlayVariants = cva(
  'pointer-events-none absolute inset-0 z-10 flex items-center justify-center rounded-lg border-2 border-dashed border-accent bg-bg text-accent uppercase [letter-spacing:0.03em]',
)
const actionRowVariants = cva('flex items-end gap-2')
const textareaVariants = cva('min-h-composer max-h-composer min-w-0 flex-1 resize-none overflow-y-auto rounded-3xl bg-glass2 p-3 text-lg leading-5 text-fg outline-none glass')
const hiddenInputVariants = cva('hidden')

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

export function PostComposer(props: { mode?: 'reply' | 'thread'; slug?: string; editingUid?: string | null; onEditDone?: () => void; nav?: JSX.Element }) {
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
  const [optionsOpen, setOptionsOpen] = createSignal(draft.optionsOpen)
  const [files, setFiles] = createSignal<File[]>(draft.files)
  const [maxMedia, setMaxMedia] = createSignal<number | null>(null)
  const [secretRefs, setSecretRefs] = createSignal<Map<number, PostPartInput> | null>(null)
  const [withFlag, setWithFlag] = createSignal(draft.withFlag)
  const [stripExifOn, setStripExifOn] = createSignal(draft.stripExifOn)
  const [dragOver, setDragOver] = createSignal(false)

  const submitter = createPostSubmit({
    onError: (e) => reportError(parseMoveAbort(e)?.message ?? String(e)),
  })

  const token = {}
  let textarea: HTMLTextAreaElement | undefined
  let fileInput: HTMLInputElement | undefined
  let root: HTMLDivElement | undefined
  let pendingCaret: number | null = null

  const roles = createMemo(() => {
    const info = store.info()
    if (!info) return []
    const master = getSecretKeyBytes()
    const forumIdBytes = fromHex(info.forumId)
    const opUid = store.page()?.uid
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

  createEffect(() => {
    const el = textarea
    const value = text()
    if (!el) return
    el.style.height = 'auto'
    const height = el.scrollHeight
    if (height > 0) el.style.height = `${height}px`
    void value
  })

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
    let cursor: number
    if (tag === 'url') {
      next = before + `[url=]${selected}[/url]` + after
      cursor = start + 5
    } else if (selected) {
      next = before + `[${tag}]${selected}[/${tag}]` + after
      cursor = start + tag.length + 2 + selected.length + tag.length + 3
    } else {
      next = before + `[${tag}][/${tag}]` + after
      cursor = start + tag.length + 2
    }
    setText(next)
    pendingCaret = cursor
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
      { text: text(), name: name(), withFlag: withFlag(), files: files(), stripExif: stripExifOn() },
    )
    if (!res) return
    setText('')
    setName('')
    setFiles([])
    setWithFlag(false)
    setOptionsOpen(isThread())
  }

  const createThread = async () => {
    const info = store.info()
    const slug = props.slug
    if (!info || !slug) return
    const role = roles().find((item) => item.kind === sendAsKind())
    const res = await submitter.createThread(
      { forumId: info.forumId, nonceShardsId: info.nonceShardsId, relayUrl: info.relayUrl, boardUid: info.boardUid, slug, role },
      { text: text(), name: name(), withFlag: withFlag(), files: files(), stripExif: stripExifOn() },
      subject(),
    )
    if (!res) return
    setText('')
    setSubject('')
    setName('')
    setFiles([])
    setWithFlag(false)
    setOptionsOpen(isThread())
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
      {props.nav}
      <Show when={dragOver()}>
        <div class={dropOverlayVariants()}>{t('postForm.dropHere')}</div>
      </Show>
      <Show when={optionsOpen()}>
        <OptionsPanel
          isEdit={isEdit()}
          isThread={isThread()}
          name={name()}
          onNameChange={setName}
          sendAsKind={sendAsKind()}
          onSendAsKindChange={setSendAsKind}
          roles={roles()}
          subject={subject()}
          onSubjectChange={setSubject}
          withFlag={withFlag()}
          onWithFlagChange={setWithFlag}
          stripExifOn={stripExifOn()}
          onStripExifChange={setStripExifOn}
          onInsertTag={insertTag}
        />
      </Show>
      <Show when={files().length > 0}>
        <FilePreview files={files()} previewUrls={previewUrls()} onRemove={removeFile} />
      </Show>
      <div class={actionRowVariants()}>
        <input
          ref={fileInput}
          type="file"
          accept="image/*,video/mp4,video/webm,audio/mpeg,audio/ogg,application/pdf"
          multiple
          class={hiddenInputVariants()}
          onChange={(e) => {
            const selected = e.currentTarget.files
            if (selected) addFiles(selected)
          }}
        />
        <Show when={!isEdit() && !isThread()}>
          <IconButton size="lg" variant="glass" active={optionsOpen()} title={t('postForm.options')} onClick={() => setOptionsOpen(!optionsOpen())}>
            <Show when={optionsOpen()} fallback={<ChevronUpIcon />}>
              <ChevronDownIcon />
            </Show>
          </IconButton>
        </Show>
        <Show when={!isEdit()}>
          <IconButton size="lg" variant="glass" title={t('postForm.addFile')} onClick={() => fileInput?.click()}>
            <ClipIcon />
          </IconButton>
        </Show>
        <textarea
          ref={textarea}
          class={textareaVariants()}
          placeholder={isThread() ? t('postForm.commentOp') : t('postForm.comment')}
          rows={1}
          value={text()}
          onInput={(e) => setText(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
              e.preventDefault()
              if (canSend()) submit()
            }
          }}
        />
        <Show when={isEdit()}>
          <IconButton size="lg" variant="glass" title={t('mod.cancel')} onClick={() => props.onEditDone?.()}>
            <XIcon width={18} height={18} />
          </IconButton>
        </Show>
        <IconButton
          size="lg"
          variant="glass"
          disabled={!canSend()}
          onClick={submit}
          title={isEdit() ? t('mod.save') : t('postForm.submit')}
        >
          <Show when={isEdit() ? ea.busy() : submitter.creating()} fallback={<SendIcon />}>
            <span class={spinnerVariants()} />
          </Show>
        </IconButton>
      </div>
    </div>
  )
}
