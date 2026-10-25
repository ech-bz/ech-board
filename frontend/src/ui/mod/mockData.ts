import type { ModActionEntry, ModFile, ModMedia, ModPost, ModReport } from './types'

const MIN = 60_000

const TEXTS = [
  'I have been working on this piece for a while and wanted to share the latest version. The lighting still feels a little uneven, but the overall composition is finally coming together. Any feedback is welcome, especially on the background and color balance.',
  'I put together a collection of references and could use some feedback. Does anyone know the source of this artwork?',
  'A short post with no attachments.',
  'I tried a different approach this time and kept the palette fairly restrained. The first version had more detail, but it made the focal point harder to read. This one feels simpler and more balanced, although I am still not sure about the contrast in the lower half. I would appreciate any suggestions before I call it finished.',
  'This thread has gone off topic again. Let us get back to the subject.',
  'I uploaded it to a file host; the link is in the previous post.',
  'Spam and advertising for an external website. Follow the link to learn more!',
  'What do you think of the new release? I think it turned out better than expected. The plot drags a little around the middle, but the visuals make up for it.',
]

const REASONS = [
  'Spam or advertising for an external website',
  'Off-topic content in a board-specific thread',
  'Personal information visible in an image',
  'Violation of the NSFW rule',
  'Repeated identical messages',
  'Thread flooding',
  'Unlabeled advertising',
]

const FILE_NAMES = ['reference', 'sketch', 'screenshot', 'wallpaper', 'meme', 'photo', 'scan', 'render']

export function uidOf(i: number): string {
  return 'UID0×' + (0x1a2b + i * 977).toString(16).toUpperCase().padStart(4, '0') + '…'
}

export function art(i: number, size = 320): string {
  const h = (i * 47) % 360
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='${size}' height='${size}' viewBox='0 0 100 100'>` +
    `<defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='hsl(${(h + 200) % 360} 55% 18%)'/><stop offset='1' stop-color='hsl(${(h + 160) % 360} 60% 38%)'/></linearGradient></defs>` +
    `<rect width='100' height='100' fill='url(#g)'/>` +
    `<circle cx='${20 + ((i * 13) % 60)}' cy='${30 + ((i * 7) % 50)}' r='22' fill='hsl(${h} 70% 55%)'/>` +
    `<circle cx='${70 - ((i * 5) % 40)}' cy='${75 - ((i * 11) % 40)}' r='28' fill='hsl(${(h + 90) % 360} 65% 50%)' fill-opacity='.85'/>` +
    `</svg>`
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`
}

function mediaOf(seed: number, kind: 'img' | 'gif' | 'vid', n: number): ModMedia {
  const ext = kind === 'img' ? ['jpg', 'png', 'webp'][seed % 3] : kind === 'gif' ? 'gif' : 'mp4'
  const mime = kind === 'img' ? `image/${ext === 'jpg' ? 'jpeg' : ext}` : kind === 'gif' ? 'image/gif' : 'video/mp4'
  return {
    id: `m${n}`,
    name: `${1790000000 + seed * 37321}.${ext}`,
    mime,
    size: kind === 'vid' ? (3000 + seed * 87) * 1024 : (120 + ((seed * 213) % 2600)) * 1024,
    width: 800,
    height: 800,
    thumbUrl: art(seed + 3, 240),
    fullUrl: art(seed + 3, 800),
    hidden: false,
  }
}

export function makePosts(boards: string[], now: number): ModPost[] {
  const slugs = boards.length ? boards : ['b']
  return Array.from({ length: 80 }, (_, i) => {
    const board = slugs[i % slugs.length]
    const op = i % 9 === 8
    const num = 6000 - i * 3
    const threadNum = op ? num : num - ((i % 9) + 1) * 3
    const withMedia = i % 3 === 0
    const gallery = i % 11 === 4 ? 4 : 0
    const media = withMedia || gallery ? Array.from({ length: gallery || 1 }, (_, k) => mediaOf(i * 5 + k, i % 7 === 3 ? 'gif' : 'img', i * 10 + k)) : []
    return {
      num,
      board,
      threadNum,
      op,
      authorUid: uidOf(i % 13),
      role: i % 17 === 1 ? 'forum_mod' : i % 23 === 2 ? 'mop' : null,
      ts: now - i * 7 * MIN,
      text: TEXTS[i % TEXTS.length],
      replyTo: !op && i % 4 === 1 ? threadNum : null,
      media,
      deleted: false,
    }
  })
}

export function makeReports(posts: ModPost[], now: number): ModReport[] {
  return Array.from({ length: 16 }, (_, i) => {
    const post = posts[(i * 5) % posts.length]
    return {
      id: `r${i}`,
      reporterUid: uidOf(i * 3 + 1),
      reason: REASONS[i % REASONS.length],
      board: post.board,
      threadNum: post.threadNum,
      postNum: post.num,
      ts: now - i * 4 * MIN,
      status: i % 6 === 5 ? 'resolved' : i % 6 === 4 ? 'ignored' : 'open',
    }
  })
}

const MOD_UIDS = ['UID0×91AC…', 'UID0×44F0…', 'UID0×7D12…']

export function makeActions(posts: ModPost[], now: number): ModActionEntry[] {
  const rows: Pick<ModActionEntry, 'kind' | 'target' | 'place' | 'detail'>[] = [
    { kind: 'ban', target: 'user', place: 'forum', detail: '(General Rule 1)' },
    { kind: 'ban', target: 'user', place: 'board', detail: '(Thread wipe)' },
    { kind: 'delete', target: 'post', place: 'thread', detail: '' },
    { kind: 'hide_media', target: 'post', place: 'thread', detail: '' },
    { kind: 'delete', target: 'thread', place: 'board', detail: '' },
    { kind: 'restore', target: 'post', place: 'thread', detail: '' },
  ]
  return Array.from({ length: 24 }, (_, i) => {
    const row = rows[i % rows.length]
    const post = posts[(i * 3 + 2) % posts.length]
    return {
      id: `seed${i}`,
      modUid: MOD_UIDS[i % MOD_UIDS.length],
      ...row,
      board: row.place === 'forum' ? null : post.board,
      threadNum: row.place === 'thread' ? post.threadNum : null,
      postNum: post.num,
      ts: now - (40 + i * 3) * MIN,
      undone: false,
    }
  })
}

export function makeFiles(boards: string[], now: number): ModFile[] {
  const slugs = boards.length ? boards : ['b']
  return Array.from({ length: 240 }, (_, i) => {
    const kind = i % 7 === 3 ? 'gif' : i % 5 === 2 ? 'vid' : 'img'
    const media = mediaOf(i * 2, kind, i)
    return {
      id: `f${i}`,
      name: `${FILE_NAMES[i % FILE_NAMES.length]}_${media.name}`,
      mime: media.mime,
      size: media.size,
      width: media.width,
      height: media.height,
      thumbUrl: media.thumbUrl,
      fullUrl: media.fullUrl,
      board: slugs[i % slugs.length],
      threadNum: 5900 - (i % 24) * 12,
      postNum: 6000 - i * 3,
      authorUid: uidOf(i % 9),
      ts: now - i * 41 * MIN,
      deleted: false,
    }
  })
}
