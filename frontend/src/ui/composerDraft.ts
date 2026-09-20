import type { RoleKind } from '../core/intent/roles'

export interface ComposerDraft {
  text: string
  subject: string
  name: string
  sendAsKind: RoleKind
  optionsOpen: boolean
  files: File[]
  withFlag: boolean
  stripExifOn: boolean
}

function emptyDraft(mode: 'reply' | 'thread'): ComposerDraft {
  return { text: '', subject: '', name: '', sendAsKind: 'anonymous', optionsOpen: mode === 'thread', files: [], withFlag: false, stripExifOn: true }
}

const drafts: Record<'reply' | 'thread', ComposerDraft> = {
  reply: emptyDraft('reply'),
  thread: emptyDraft('thread'),
}

export function getComposerDraft(mode: 'reply' | 'thread'): ComposerDraft {
  return drafts[mode]
}

export function saveComposerDraft(mode: 'reply' | 'thread', draft: ComposerDraft): void {
  drafts[mode] = draft
}
