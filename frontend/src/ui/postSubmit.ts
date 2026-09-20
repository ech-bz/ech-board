import { createSignal, type Accessor } from 'solid-js'
import { submitNewThread, submitReply } from '../core/intent/postSubmit'
import type { PostFormState, PostSubmitCtx } from '../core/intent/postSubmit'

export type { PostFormState } from '../core/intent/postSubmit'

export interface PostSubmit {
  creating: Accessor<boolean>
  progress: Accessor<number | null>
  createThread: (ctx: PostSubmitCtx, form: PostFormState, subject: string) => Promise<unknown>
  createReply: (
    ctx: PostSubmitCtx & { threadUid: string; postMap?: Map<number, { id: Uint8Array; author: Uint8Array }> },
    form: PostFormState,
  ) => Promise<unknown>
  cancel: () => void
}

export function createPostSubmit(options: { onError: (e: unknown) => void }): PostSubmit {
  const [creating, setCreating] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)
  let controller: AbortController | null = null

  const onUploadProgress = (loaded: number, total: number) => {
    if (total > 0) setProgress(loaded / total)
  }

  const cancel = () => {
    controller?.abort()
  }

  const createThread = async (ctx: PostSubmitCtx, form: PostFormState, subject: string) => {
    if (!form.files.length && !form.text.trim()) return null
    if (!ctx.nonceShardsId) return null
    const current = new AbortController()
    controller = current
    setCreating(true)
    try {
      return await submitNewThread(ctx, form, subject, onUploadProgress, current.signal)
    } catch (e) {
      if (!current.signal.aborted) options.onError(e)
      return null
    } finally {
      if (controller === current) controller = null
      setCreating(false)
      setProgress(null)
    }
  }

  const createReply = async (
    ctx: PostSubmitCtx & { threadUid: string; postMap?: Map<number, { id: Uint8Array; author: Uint8Array }> },
    form: PostFormState,
  ) => {
    if (!form.files.length && !form.text.trim()) return null
    if (!ctx.nonceShardsId) return null
    const current = new AbortController()
    controller = current
    setCreating(true)
    try {
      return await submitReply(ctx, form, onUploadProgress, current.signal)
    } catch (e) {
      if (!current.signal.aborted) options.onError(e)
      return null
    } finally {
      if (controller === current) controller = null
      setCreating(false)
      setProgress(null)
    }
  }

  return { creating, progress, createThread, createReply, cancel }
}
