import { PostProjection, ThreadProjection, PostPartVec, type PostObject, type PostPartInput } from './bcs/types'
import { toHex } from './intent/crypto'

export function partsToPlainText(parts: PostPartInput[]): string {
  let out = ''
  for (const p of parts) out += partText(p)
  return out
}

function partText(p: PostPartInput): string {
  if ('Plain' in p) return p.Plain
  if ('Bold' in p) return partsToPlainText(p.Bold)
  if ('Italic' in p) return partsToPlainText(p.Italic)
  if ('Code' in p) return partsToPlainText(p.Code)
  if ('Underline' in p) return partsToPlainText(p.Underline)
  if ('Overline' in p) return partsToPlainText(p.Overline)
  if ('Spoiler' in p) return partsToPlainText(p.Spoiler)
  if ('Strike' in p) return partsToPlainText(p.Strike)
  if ('Sup' in p) return partsToPlainText(p.Sup)
  if ('Sub' in p) return partsToPlainText(p.Sub)
  if ('Link' in p) return partsToPlainText(p.Link.children)
  if ('ReplyTo' in p) return `>>${p.ReplyTo.hint}`
  if ('Secret' in p) return '[secret]'
  return ''
}

export function partsToEditText(parts: PostPartInput[]): { text: string; secrets: Map<number, PostPartInput> } {
  const secrets = new Map<number, PostPartInput>()
  let n = 0
  const render = (ps: PostPartInput[]): string => {
    let out = ''
    for (const p of ps) {
      if ('Plain' in p) out += p.Plain
      else if ('Bold' in p) out += `[b]${render(p.Bold)}[/b]`
      else if ('Italic' in p) out += `[i]${render(p.Italic)}[/i]`
      else if ('Code' in p) out += `[code]${render(p.Code)}[/code]`
      else if ('Underline' in p) out += `[u]${render(p.Underline)}[/u]`
      else if ('Overline' in p) out += `[o]${render(p.Overline)}[/o]`
      else if ('Spoiler' in p) out += `[spoiler]${render(p.Spoiler)}[/spoiler]`
      else if ('Strike' in p) out += `[s]${render(p.Strike)}[/s]`
      else if ('Sup' in p) out += `[sup]${render(p.Sup)}[/sup]`
      else if ('Sub' in p) out += `[sub]${render(p.Sub)}[/sub]`
      else if ('Link' in p) out += `[url=${p.Link.url}]${render(p.Link.children)}[/url]`
      else if ('ReplyTo' in p) out += `>>${p.ReplyTo.hint}`
      else if ('Secret' in p) {
        n += 1
        secrets.set(n, p)
        out += `[secret_${n}]`
      }
    }
    return out
  }
  return { text: render(parts), secrets }
}

const partsCache = new WeakMap<Uint8Array, PostPartInput[]>()

export interface PostMeta {
  uid: string
  number: number
  deleted: boolean
  textHash: string | null
}

const metaCache = new WeakMap<PostObject, PostMeta>()

export function postMeta(post: PostObject): PostMeta {
  const hit = metaCache.get(post)
  if (hit) return hit
  const pp = new PostProjection(post.projection)
  const th = pp.text_hash()
  const meta: PostMeta = { uid: toHex(post.root.id), number: Number(pp.number()), deleted: pp.deleted(), textHash: th ? toHex(th) : null }
  metaCache.set(post, meta)
  return meta
}

export function postUid(post: PostObject): string {
  return postMeta(post).uid
}

export function postParts(post: PostObject, contentMap: Map<string, Uint8Array>): PostPartInput[] {
  const pp = new PostProjection(post.projection)
  const th = pp.text_hash()
  if (!th) return []
  const bytes = contentMap.get(toHex(th))
  if (!bytes) return []
  const cached = partsCache.get(bytes)
  if (cached) return cached
  try {
    const parts = PostPartVec.parse(bytes)
    partsCache.set(bytes, parts)
    return parts
  } catch {
    return []
  }
}

export interface PostRef {
  postUid: string
  number: number
}

export function postReplyTargets(parts: PostPartInput[]): string[] {
  const found = new Set<string>()
  collectReplyTargets(parts, found)
  return Array.from(found)
}

function collectReplyTargets(parts: PostPartInput[], out: Set<string>): void {
  for (const p of parts) {
    if ('ReplyTo' in p) out.add(toHex(p.ReplyTo.post))
    else if ('Bold' in p) collectReplyTargets(p.Bold, out)
    else if ('Italic' in p) collectReplyTargets(p.Italic, out)
    else if ('Code' in p) collectReplyTargets(p.Code, out)
    else if ('Underline' in p) collectReplyTargets(p.Underline, out)
    else if ('Overline' in p) collectReplyTargets(p.Overline, out)
    else if ('Spoiler' in p) collectReplyTargets(p.Spoiler, out)
    else if ('Strike' in p) collectReplyTargets(p.Strike, out)
    else if ('Sup' in p) collectReplyTargets(p.Sup, out)
    else if ('Sub' in p) collectReplyTargets(p.Sub, out)
    else if ('Link' in p) collectReplyTargets(p.Link.children, out)
  }
}

export function threadCount(tp: ThreadProjection): number {
  const pd = tp.posts_deleted()
  return pd != null ? Number(tp.posts().counter) - Number(pd) : Number(tp.posts().counter)
}

export function threadTopicText(tp: ThreadProjection, content: Map<string, Uint8Array>): string {
  const th = tp.topic_hash()
  if (!th) return ''
  const bytes = content.get(toHex(th))
  return bytes ? new TextDecoder().decode(bytes) : ''
}

export function deriveThreadTitle(tp: ThreadProjection, content: Map<string, Uint8Array>, op: PostObject | null | undefined): string {
  const topic = threadTopicText(tp, content)
  if (topic) return topic
  if (op) {
    const pp = new PostProjection(op.projection)
    const oh = pp.text_hash()
    if (oh) {
      const ob = content.get(toHex(oh))
      if (ob) {
        const parts = PostPartVec.parse(ob)
        const line = partsToPlainText(parts).split('\n').find((s) => s.trim())
        if (line) return line
      }
    }
  }
  return ''
}
