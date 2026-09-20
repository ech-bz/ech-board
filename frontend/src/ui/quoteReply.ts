import { onCleanup } from 'solid-js'

function tagOf(el: Element): string | null {
  const t = el.tagName
  if (t === 'B') return 'b'
  if (t === 'I') return 'i'
  if (t === 'U') return 'u'
  if (t === 'S') return 's'
  if (t === 'SUP') return 'sup'
  if (t === 'SUB') return 'sub'
  if (t === 'PRE') return 'code'
  if (el.classList.contains('post-spoiler')) return 'spoiler'
  if (el.classList.contains('post-overline')) return 'o'
  return null
}

function textNodesInRange(range: Range): Text[] {
  const root = range.commonAncestorContainer
  if (root.nodeType === Node.TEXT_NODE) return [root as Text]
  const out: Text[] = []
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let n: Node | null
  while ((n = walker.nextNode())) {
    if (range.intersectsNode(n)) out.push(n as Text)
  }
  return out
}

function selectionToBb(sel: Selection): string {
  const out: string[] = []
  for (let i = 0; i < sel.rangeCount; i++) {
    const range = sel.getRangeAt(i)
    const runs: Array<{ tags: string[]; text: string }> = []
    for (const node of textNodesInRange(range)) {
      let text = node.data
      if (node === range.startContainer && node === range.endContainer) {
        text = text.slice(range.startOffset, range.endOffset)
      } else {
        if (node === range.startContainer) text = text.slice(range.startOffset)
        if (node === range.endContainer) text = text.slice(0, range.endOffset)
      }
      if (!text) continue
      const stack: string[] = []
      let el: Element | null = node.parentElement
      while (el) {
        const tag = tagOf(el)
        if (tag) stack.unshift(tag)
        el = el.parentElement
      }
      const key = stack.join('\u0000')
      const last = runs[runs.length - 1]
      if (last && last.tags.join('\u0000') === key) {
        last.text += text
      } else {
        runs.push({ tags: stack, text })
      }
    }
    for (const run of runs) {
      const opens = run.tags.map((t) => `[${t}]`).join('')
      const closes = [...run.tags].reverse().map((t) => `[/${t}]`).join('')
      out.push(opens + run.text + closes)
    }
  }
  return out.join('')
}

export function createQuoteReply() {
  let selectedText = ''
  const onSelectionChange = () => {
    const sel = window.getSelection()
    if (!sel || sel.isCollapsed || sel.rangeCount === 0 || !sel.toString().trim()) {
      selectedText = ''
      return
    }
    const node = sel.anchorNode
    const el = node && node.nodeType === Node.TEXT_NODE ? node.parentElement : (node as Element | null)
    if (el && el.closest('input, textarea, [contenteditable]')) return
    const text = selectionToBb(sel)
    if (text && text.trim()) selectedText = text
  }
  document.addEventListener('selectionchange', onSelectionChange)
  onCleanup(() => document.removeEventListener('selectionchange', onSelectionChange))

  const takeQuote = (): string => {
    const sel = selectedText.trim()
    selectedText = ''
    if (!sel) return ''
    return sel
      .split('\n')
      .filter((line) => line.trim() !== '')
      .map((line) => `> ${line}`)
      .join('\n')
  }

  return { takeQuote }
}
