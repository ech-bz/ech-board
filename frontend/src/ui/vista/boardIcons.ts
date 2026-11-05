const BASE = import.meta.env.BASE_URL

export const VISTA_ICON_BASE = `${BASE}static/vista/icons/`
export const VISTA_LOGO = `${BASE}static/vista/logo.png`
export const DEFAULT_LOGO = `${BASE}static/logo.svg`

const BY_SLUG: Record<string, string> = {
  b: 'bomb',
  d: 'headphones',
  ai: 'monitor',
  mad: 'dice',
  paranormal: 'bug',
  pa: 'bug',
  test: 'mail',
  meta: 'users',
  draw: 'palette',
  art: 'palette',
  ad: 'phone',
  ads: 'phone',
  otaku: 'amor',
  archive: 'floppy',
}

const BY_TITLE: Array<[RegExp, string]> = [
  [/бред|random/i, 'bomb'],
  [/искусствен|нейро|\bai\b/i, 'monitor'],
  [/дневч|дневник|diary|музык|music/i, 'headphones'],
  [/мэд|мэдач|mad|игр|game/i, 'dice'],
  [/сверхъ|паранорм|мистик|paranormal|supernatural/i, 'bug'],
  [/йцукен|тест|test/i, 'mail'],
  [/дискусс|борд|мета|meta|discuss/i, 'users'],
  [/рисов|рисун|арт|draw|art/i, 'palette'],
  [/саморекл|реклам|advert|promo/i, 'phone'],
  [/отаку|аниме|otaku|anime/i, 'amor'],
  [/архив|archive/i, 'floppy'],
  [/видео|video|кино|movie/i, 'video'],
  [/звук|аудио|audio/i, 'audio'],
  [/новост|news/i, 'news'],
  [/интернет|network|сеть/i, 'network'],
]

const FALLBACK = ['folder', 'globe', 'chat', 'news', 'network', 'folder-pictures', 'games', 'palette', 'business', 'eyes']

function hash(value: string): number {
  let h = 0
  for (let i = 0; i < value.length; i++) h = (h * 31 + value.charCodeAt(i)) >>> 0
  return h
}

export function vistaBoardIconName(slug: string, title?: string): string {
  const bySlug = BY_SLUG[slug.toLowerCase()]
  if (bySlug) return bySlug
  if (title) {
    for (const [pattern, name] of BY_TITLE) if (pattern.test(title)) return name
  }
  return FALLBACK[hash(slug) % FALLBACK.length]
}

export function vistaBoardIcon(slug: string, title?: string): string {
  return `${VISTA_ICON_BASE}${vistaBoardIconName(slug, title)}.png`
}
