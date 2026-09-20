import { createMemo, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { toHex } from '../../core/intent/crypto'
import { fetchPostView } from '../../core/api/relay'
import { PostView as PostViewBcs } from '../../core/bcs/relay'
import { BoardProjection, ThreadProjection, PostPartVec, type PostPartInput } from '../../core/bcs/types'
import { scrollToPost } from '../scroll'
import { threadPath, type Route } from '../route'
import { useNav } from '../nav'
import { SecretSpan } from './SecretSpan'
import { PostRefLink } from '../postTooltip/PostRefLink'
import type { DecryptContext } from '../../core/intent/decrypt'

const quoteVariants = cva('text-quote')
const codeBlockVariants = cva('my-1.5 block overflow-x-auto whitespace-pre-wrap break-words rounded-sm bg-panel2 p-2.5 font-mono')
const overlineVariants = cva('overline')
const spoilerVariants = cva('post-spoiler')
const linkCardVariants = cva('inline-flex h-linkcard w-linkcard cursor-pointer items-center justify-center overflow-hidden break-words rounded-sm bg-panel2 p-1.5 text-center font-semibold text-accent [text-decoration:none] hover:bg-hover')
const linkRowVariants = cva('inline-flex flex-wrap gap-1.5')
const refLinkVariants = cva('cursor-pointer text-accent hover:text-accent-hover')

const URL_RE = /https?:\/\/[^\s<>"')\]}]+/g
const URL_TRAIL = /[.,;:!?]+$/

function isExternal(url: string): boolean {
  try {
    return new URL(url, window.location.href).host !== window.location.host
  } catch {
    return true
  }
}

function renderTextWithLinks(text: string): JSX.Element[] {
  const out: JSX.Element[] = []
  let last = 0
  URL_RE.lastIndex = 0
  for (let m = URL_RE.exec(text); m !== null; m = URL_RE.exec(text)) {
    const url = m[0].replace(URL_TRAIL, '')
    const suffix = m[0].slice(url.length)
    if (m.index > last) out.push(text.slice(last, m.index))
    out.push(
      <a href={url} {...(isExternal(url) ? { target: '_blank', rel: 'noopener noreferrer' } : {})}>
        {url}
      </a>,
    )
    if (suffix) out.push(suffix)
    last = m.index + m[0].length
  }
  if (last < text.length) out.push(text.slice(last))
  return out
}

function extractCodeText(parts: PostPartInput[]): string {
  let s = ''
  for (const p of parts) {
    if ('Plain' in p) s += p.Plain
    else if ('Code' in p) s += extractCodeText(p.Code)
  }
  return s.replace(/^\n+/, '').replace(/\n+$/, '')
}

function isReplyRefLine(line: string): boolean {
  if (!line.startsWith('>>')) return false
  let i = 2
  while (i < line.length && /[A-Za-z0-9/:._-]/.test(line[i])) i++
  let raw = line.slice(2, i)
  while (raw.length > 0 && !/[A-Za-z0-9]/.test(raw[raw.length - 1])) raw = raw.slice(0, -1)
  const parts = raw.split('/')
  if (parts.length > 1 && parts[0] === '') parts.shift()
  if (parts.length < 1 || parts.length > 3) return false
  return /^\d+$/.test(parts[parts.length - 1])
}

function sameBytes(a: Uint8Array | undefined, b: Uint8Array | undefined): boolean {
  if (a === b) return true
  if (!a || !b || a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
  return true
}

interface RenderOpts {
  masterSecret: Uint8Array
  forumId: Uint8Array
  senderTweak: Uint8Array
  senderPub: Uint8Array
  postAuthors: Map<string, Uint8Array>
  myAuthorTweaks: Map<string, Uint8Array>
  postNums: Map<string, number>
  relayUrl: string
  navigate: (route: Route) => void
  forumIdHex: string
  boardUid: string
  slug?: string
  threadNum?: number
}

type RenderNested = (bytes: Uint8Array) => JSX.Element

function renderPart(p: PostPartInput, opts: RenderOpts, replyTweaks: Uint8Array[], missingRefs: Uint8Array[], renderNested: RenderNested): JSX.Element {
  if ('Plain' in p) {
    const lines = p.Plain.split('\n')
    return (
      <span>
        {lines.map((line, li) => (
          <>
            {line.startsWith('>') && !isReplyRefLine(line) ? <span class={quoteVariants()}>{renderTextWithLinks(line)}</span> : renderTextWithLinks(line)}
            {li < lines.length - 1 ? '\n' : null}
          </>
        ))}
      </span>
    )
  }
  if ('Bold' in p) return <b>{renderParts(p.Bold, opts, replyTweaks, missingRefs, renderNested)}</b>
  if ('Italic' in p) return <i>{renderParts(p.Italic, opts, replyTweaks, missingRefs, renderNested)}</i>
  if ('Code' in p) return <pre class={codeBlockVariants()}>{extractCodeText(p.Code)}</pre>
  if ('Underline' in p) return <u>{renderParts(p.Underline, opts, replyTweaks, missingRefs, renderNested)}</u>
  if ('Overline' in p) return <span class={overlineVariants()}>{renderParts(p.Overline, opts, replyTweaks, missingRefs, renderNested)}</span>
  if ('Spoiler' in p) return <span class={spoilerVariants()}>{renderParts(p.Spoiler, opts, replyTweaks, missingRefs, renderNested)}</span>
  if ('Strike' in p) return <s>{renderParts(p.Strike, opts, replyTweaks, missingRefs, renderNested)}</s>
  if ('Sup' in p) return <sup>{renderParts(p.Sup, opts, replyTweaks, missingRefs, renderNested)}</sup>
  if ('Sub' in p) return <sub>{renderParts(p.Sub, opts, replyTweaks, missingRefs, renderNested)}</sub>
  if ('ReplyTo' in p) {
    const postHex = toHex(p.ReplyTo.post)
    const localNum = opts.postNums.get(postHex)
    if (localNum != null) {
      const localHref = opts.slug && opts.threadNum != null ? threadPath(opts.forumIdHex, opts.slug, opts.threadNum, localNum) : `#post-${localNum}`
      return (
        <PostRefLink
          class={refLinkVariants()}
          href={localHref}
          label={`>>${localNum}`}
          target={{ postUid: postHex, number: localNum }}
          onClick={(e) => {
            e.preventDefault()
            if (opts.slug != null && opts.threadNum != null) opts.navigate({ page: 'thread', forumId: opts.forumIdHex, slug: opts.slug, threadNum: opts.threadNum, postNum: localNum })
            else scrollToPost(localNum)
          }}
        />
      )
    }
    const hint = p.ReplyTo.hint
    const slash = hint.lastIndexOf('/')
    const numStr = slash >= 0 ? hint.slice(slash + 1) : hint
    const num = /^\d+$/.test(numStr) ? Number(numStr) : undefined
    const sameBoard = toHex(p.ReplyTo.board) === opts.boardUid
    const sameForum = toHex(p.ReplyTo.forum).replace(/^0x/, '') === opts.forumIdHex
    if (sameForum && num != null) {
      const label = sameBoard ? `>>${num}` : `>>/${hint}`
      return (
        <PostRefLink
          class={refLinkVariants()}
          label={label}
          target={{ postUid: postHex, number: num }}
          onClick={(e) => {
            e.preventDefault()
            void (async () => {
              try {
                const buf = await fetchPostView(opts.relayUrl, postHex)
                const view = PostViewBcs.parse(new Uint8Array(buf))
                const slug = new BoardProjection(view.board.projection).slug()
                const threadNum = Number(new ThreadProjection(view.thread.projection).number())
                opts.navigate({ page: 'thread', forumId: opts.forumIdHex, slug, threadNum, postNum: num })
              } catch {
                return
              }
            })()
          }}
        />
      )
    }
    return <span class={refLinkVariants()}>{`>>${hint}`}</span>
  }
  if ('Secret' in p) {
    return (
      <SecretSpan
        dataNonce={new Uint8Array(p.Secret.data_nonce)}
        dataCt={new Uint8Array(p.Secret.data_ct)}
        encryptedKeys={p.Secret.encrypted_keys.map((ek) => ({ nonce: new Uint8Array(ek.nonce), ct: new Uint8Array(ek.ct), tag: new Uint8Array(ek.tag) }))}
        senderTweak={opts.senderTweak}
        senderPub={opts.senderPub}
        masterSecret={opts.masterSecret}
        forumId={opts.forumId}
        replyTweaks={replyTweaks}
        missingRefs={missingRefs}
        relayUrl={opts.relayUrl}
        renderNested={renderNested}
      />
    )
  }
  return null
}

function renderLink(p: Extract<PostPartInput, { Link: unknown }>, opts: RenderOpts, replyTweaks: Uint8Array[], missingRefs: Uint8Array[], renderNested: RenderNested): JSX.Element {
  return (
    <a class={linkCardVariants()} href={p.Link.url} {...(isExternal(p.Link.url) ? { target: '_blank', rel: 'noopener noreferrer' } : {})}>
      {renderParts(p.Link.children, opts, replyTweaks, missingRefs, renderNested)}
    </a>
  )
}

function collectInto(sub: { tweaks: Uint8Array[]; missing: Uint8Array[] }, tweaks: Uint8Array[], missing: Uint8Array[]) {
  tweaks.push(...sub.tweaks)
  missing.push(...sub.missing)
}

function collectReplyTweaks(parts: PostPartInput[], opts: RenderOpts): { tweaks: Uint8Array[]; missing: Uint8Array[] } {
  const tweaks: Uint8Array[] = []
  const missing: Uint8Array[] = []
  for (const p of parts) {
    if ('ReplyTo' in p) {
      const author = opts.postAuthors.get(toHex(p.ReplyTo.post))
      if (author) {
        const tweak = opts.myAuthorTweaks.get(toHex(author))
        if (tweak) tweaks.push(tweak)
      } else {
        missing.push(new Uint8Array(p.ReplyTo.post))
      }
    }
    if ('Bold' in p) collectInto(collectReplyTweaks(p.Bold, opts), tweaks, missing)
    if ('Italic' in p) collectInto(collectReplyTweaks(p.Italic, opts), tweaks, missing)
    if ('Code' in p) collectInto(collectReplyTweaks(p.Code, opts), tweaks, missing)
    if ('Underline' in p) collectInto(collectReplyTweaks(p.Underline, opts), tweaks, missing)
    if ('Overline' in p) collectInto(collectReplyTweaks(p.Overline, opts), tweaks, missing)
    if ('Spoiler' in p) collectInto(collectReplyTweaks(p.Spoiler, opts), tweaks, missing)
    if ('Strike' in p) collectInto(collectReplyTweaks(p.Strike, opts), tweaks, missing)
    if ('Sup' in p) collectInto(collectReplyTweaks(p.Sup, opts), tweaks, missing)
    if ('Sub' in p) collectInto(collectReplyTweaks(p.Sub, opts), tweaks, missing)
    if ('Link' in p) collectInto(collectReplyTweaks(p.Link.children, opts), tweaks, missing)
  }
  return { tweaks, missing }
}

function renderParts(parts: PostPartInput[], opts: RenderOpts, replyTweaks: Uint8Array[], missingRefs: Uint8Array[], renderNested: RenderNested): JSX.Element[] {
  const out: JSX.Element[] = []
  let i = 0
  while (i < parts.length) {
    const p = parts[i]
    if ('Link' in p) {
      let j = i + 1
      while (j < parts.length) {
        const q = parts[j]
        if ('Link' in q || ('Plain' in q && /^\r?\n$/.test(q.Plain))) j++
        else break
      }
      const prev = i > 0 ? parts[i - 1] : null
      const next = j < parts.length ? parts[j] : null
      const breakBefore = i > 0 && !(prev && 'Plain' in prev && /[\r\n]+$/.test(prev.Plain))
      const breakAfter = j < parts.length && !(next && 'Plain' in next && /^[\r\n]+/.test(next.Plain))
      if (breakBefore) out.push('\n')
      out.push(
        <span class={linkRowVariants()}>
          {parts
            .slice(i, j)
            .filter((l): l is Extract<PostPartInput, { Link: unknown }> => 'Link' in l)
            .map((l) => renderLink(l, opts, replyTweaks, missingRefs, renderNested))}
        </span>,
      )
      if (breakAfter) out.push('\n')
      i = j
    } else {
      out.push(renderPart(p, opts, replyTweaks, missingRefs, renderNested))
      i++
    }
  }
  return out
}

export function PostContent(props: {
  parts: PostPartInput[]
  senderTweak: Uint8Array
  senderPub: Uint8Array
  decryptCtx: DecryptContext
  relayUrl: string
  forumId: string
  boardUid: string
  slug?: string
  threadNum?: number
}) {
  const nav = useNav()
  const inputs = createMemo(
    () => ({
      parts: props.parts,
      senderTweak: props.senderTweak,
      senderPub: props.senderPub,
      decryptCtx: props.decryptCtx,
      relayUrl: props.relayUrl,
      forumId: props.forumId,
      boardUid: props.boardUid,
      slug: props.slug,
      threadNum: props.threadNum,
    }),
    undefined,
    {
      equals: (a, b) =>
        a.parts === b.parts &&
        sameBytes(a.senderTweak, b.senderTweak) &&
        sameBytes(a.senderPub, b.senderPub) &&
        a.decryptCtx === b.decryptCtx &&
        a.relayUrl === b.relayUrl &&
        a.forumId === b.forumId &&
        a.boardUid === b.boardUid &&
        a.slug === b.slug &&
        a.threadNum === b.threadNum,
    },
  )
  const content = createMemo(() => {
    const current = inputs()
    const opts: RenderOpts = {
      masterSecret: current.decryptCtx.masterSecret,
      forumId: current.decryptCtx.forumId,
      senderTweak: current.senderTweak,
      senderPub: current.senderPub,
      postAuthors: current.decryptCtx.postAuthors,
      myAuthorTweaks: current.decryptCtx.myAuthorTweaks,
      postNums: current.decryptCtx.postNums,
      relayUrl: current.relayUrl,
      navigate: nav.navigate,
      forumIdHex: current.forumId,
      boardUid: current.boardUid,
      slug: current.slug,
      threadNum: current.threadNum,
    }
    const collect = collectReplyTweaks(current.parts, opts)
    const renderNested: RenderNested = (nestedBytes) => {
      try {
        return renderParts(PostPartVec.parse(nestedBytes), opts, collect.tweaks, collect.missing, renderNested)
      } catch {
        return null
      }
    }
    return renderParts(current.parts, opts, collect.tweaks, collect.missing, renderNested)
  })
  return <>{content()}</>
}
