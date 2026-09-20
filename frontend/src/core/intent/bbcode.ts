import type { PostPartInput } from '../bcs/types'
import { serializePostParts } from '../bcs/types'
import { encryptSecret } from './secret'

type VariantName = 'Bold' | 'Italic' | 'Code' | 'Underline' | 'Overline' | 'Spoiler' | 'Strike' | 'Sup' | 'Sub'

const TAG_MAP: Record<string, VariantName> = {
  b: 'Bold',
  i: 'Italic',
  code: 'Code',
  u: 'Underline',
  o: 'Overline',
  spoiler: 'Spoiler',
  s: 'Strike',
  sup: 'Sup',
  sub: 'Sub',
}

export type PostRef = { forum?: string; board?: string; postNum: number }
export type ResolvedReply = { forum: Uint8Array; board: Uint8Array; post: Uint8Array; hint: string; author?: Uint8Array }
export type PostResolver = (ref: PostRef) => Promise<ResolvedReply | null>

function parsePostRef(raw: string): PostRef | null {
  const parts = raw.split('/')
  if (parts.length > 1 && parts[0] === '') parts.shift()
  let forum: string | undefined
  let board: string | undefined
  let numStr: string
  if (parts.length === 1) {
    numStr = parts[0]
  } else if (parts.length === 2) {
    board = parts[0]
    numStr = parts[1]
  } else if (parts.length === 3) {
    forum = parts[0]
    board = parts[1]
    numStr = parts[2]
  } else {
    return null
  }
  if (!/^\d+$/.test(numStr)) return null
  return { forum: forum || undefined, board: board || undefined, postNum: parseInt(numStr, 10) }
}

type ParsedPart =
  | { kind: 'plain'; text: string }
  | { kind: 'formatted'; variant: VariantName; children: ParsedPart[] }
  | { kind: 'secret'; children: ParsedPart[] }
  | { kind: 'secretRef'; index: number }
  | { kind: 'link'; url: string; children: ParsedPart[] }
  | { kind: 'reply'; forum: Uint8Array; board: Uint8Array; post: Uint8Array; hint: string; author?: Uint8Array }

export async function parseBbCode(text: string, resolvePost?: PostResolver): Promise<ParsedPart[]> {
  const result: ParsedPart[] = []
  let i = 0

  while (i < text.length) {
    if (text[i] === '>' && text[i + 1] === '>' && resolvePost) {
      const start = i + 2
      let end = start
      while (end < text.length && /[A-Za-z0-9/:._-]/.test(text[end])) end++
      let raw = text.slice(start, end)
      while (raw.length > 0 && !/[A-Za-z0-9]/.test(raw[raw.length - 1])) raw = raw.slice(0, -1)
      const ref = parsePostRef(raw)
      const resolved = ref ? await resolvePost(ref) : null
      if (resolved) {
        result.push({ kind: 'reply', forum: resolved.forum, board: resolved.board, post: resolved.post, hint: resolved.hint, author: resolved.author })
        i = end
        continue
      }
      pushPlain(result, '>>')
      i += 2
      continue
    }

    if (text[i] !== '[') {
      let end = text.indexOf('[', i)
      if (end === -1) end = text.length
      const gtGt = resolvePost ? text.indexOf('>>', i) : -1
      if (gtGt !== -1 && gtGt < end) end = gtGt
      if (end > i) {
        pushPlain(result, text.slice(i, end))
      }
      i = end
      continue
    }

    const closeBracket = text.indexOf(']', i)
    if (closeBracket === -1) {
      pushPlain(result, text.slice(i))
      break
    }

    const tag = text.slice(i + 1, closeBracket)
    const closing = tag.startsWith('/')
    const tagName = closing ? tag.slice(1) : tag

    if (closing) {
      pushPlain(result, text[i])
      i++
      continue
    }

    const secretRef = /^secret_(\d+)$/.exec(tagName)
    if (secretRef) {
      result.push({ kind: 'secretRef', index: parseInt(secretRef[1], 10) })
      i = closeBracket + 1
      continue
    }

    if (tagName === 'secret') {
      const closeTag = '[/secret]'
      const endTag = text.indexOf(closeTag, closeBracket + 1)
      if (endTag === -1) {
        pushPlain(result, text.slice(i, closeBracket + 1))
        i = closeBracket + 1
        continue
      }
      const inner = text.slice(closeBracket + 1, endTag)
      const children = await parseBbCode(inner, resolvePost)
      result.push({ kind: 'secret', children })
      i = endTag + closeTag.length
      continue
    }

    if (tagName.startsWith('url=')) {
      const url = tag.slice(4)
      const closeTag = '[/url]'
      const endTag = text.indexOf(closeTag, closeBracket + 1)
      if (endTag === -1) {
        pushPlain(result, text.slice(i, closeBracket + 1))
        i = closeBracket + 1
        continue
      }
      const inner = text.slice(closeBracket + 1, endTag)
      const children = await parseBbCode(inner, resolvePost)
      result.push({ kind: 'link', url, children })
      i = endTag + closeTag.length
      continue
    }

    const variant = TAG_MAP[tagName]
    if (!variant) {
      pushPlain(result, text[i])
      i++
      continue
    }

    const closeTag = `[/${tagName}]`
    const endTag = text.indexOf(closeTag, closeBracket + 1)
    if (endTag === -1) {
      pushPlain(result, text.slice(i, closeBracket + 1))
      i = closeBracket + 1
      continue
    }

    const inner = text.slice(closeBracket + 1, endTag)
    const children = await parseBbCode(inner, resolvePost)
    result.push({ kind: 'formatted', variant, children })
    i = endTag + closeTag.length
  }

  return result
}

function pushPlain(result: ParsedPart[], text: string) {
  const last = result[result.length - 1]
  if (last && last.kind === 'plain') {
    last.text += text
  } else {
    result.push({ kind: 'plain', text })
  }
}

export async function finalizeParts(parts: ParsedPart[], senderPriv: Uint8Array, senderPub: Uint8Array, secretRefs?: Map<number, PostPartInput>): Promise<PostPartInput[]> {
  const seen = new Set<string>()
  const replyPubkeys: Uint8Array[] = []
  collectReplyPubkeys(parts, seen, replyPubkeys)

  return convertParts(parts, senderPriv, senderPub, replyPubkeys, secretRefs)
}

function collectReplyPubkeys(parts: ParsedPart[], seen: Set<string>, replyPubkeys: Uint8Array[]) {
  for (const p of parts) {
    if (p.kind === 'reply') {
      if (p.author) {
        const hex = Array.from(p.author, b => b.toString(16).padStart(2, '0')).join('')
        if (!seen.has(hex)) {
          seen.add(hex)
          replyPubkeys.push(p.author)
        }
      }
    } else if (p.kind === 'formatted' || p.kind === 'link') {
      collectReplyPubkeys(p.children, seen, replyPubkeys)
    } else if (p.kind === 'secret') {
      collectReplyPubkeys(p.children, seen, replyPubkeys)
    }
  }
}

async function convertParts(
  parts: ParsedPart[],
  senderPriv: Uint8Array,
  senderPub: Uint8Array,
  replyPubkeys: Uint8Array[],
  secretRefs?: Map<number, PostPartInput>,
): Promise<PostPartInput[]> {
  const out: PostPartInput[] = []
  for (const p of parts) {
    if (p.kind === 'plain') {
      out.push({ Plain: p.text })
    } else if (p.kind === 'reply') {
      out.push({ ReplyTo: { forum: p.forum, board: p.board, post: p.post, hint: p.hint } })
    } else if (p.kind === 'secretRef') {
      const orig = secretRefs?.get(p.index)
      if (orig) out.push(orig)
    } else if (p.kind === 'secret') {
      const children = await convertParts(p.children, senderPriv, senderPub, replyPubkeys, secretRefs)
      const data = serializePostParts(children)
      out.push(await encryptSecret(data, senderPriv, senderPub, replyPubkeys))
    } else if (p.kind === 'link') {
      const children = await convertParts(p.children, senderPriv, senderPub, replyPubkeys, secretRefs)
      out.push({ Link: { url: p.url, children } })
    } else if (p.kind === 'formatted') {
      const children = await convertParts(p.children, senderPriv, senderPub, replyPubkeys, secretRefs)
      out.push({ [p.variant]: children } as PostPartInput)
    }
  }
  return out
}

export function firstPlainLine(parts: PostPartInput[]): string {
  for (const p of parts) {
    const s = firstPlainLineOfPart(p)
    if (s && s.trim()) return s
  }
  return ''
}

function firstPlainLineOfPart(p: PostPartInput): string | null {
  if ('Plain' in p) return p.Plain
  if ('Bold' in p) return firstPlainLine(p.Bold)
  if ('Italic' in p) return firstPlainLine(p.Italic)
  if ('Underline' in p) return firstPlainLine(p.Underline)
  if ('Overline' in p) return firstPlainLine(p.Overline)
  if ('Spoiler' in p) return firstPlainLine(p.Spoiler)
  if ('Strike' in p) return firstPlainLine(p.Strike)
  if ('Sup' in p) return firstPlainLine(p.Sup)
  if ('Sub' in p) return firstPlainLine(p.Sub)
  return null
}
