export function scrollToPost(postNum: number) {
  const el = document.getElementById(`post-${postNum}`)
  if (!el) return
  const header = [...document.querySelectorAll<HTMLElement>('.desktop-top, .mobile-top')].find((h) => h.offsetHeight > 0)
  el.style.scrollMarginTop = `${(header?.offsetHeight ?? 0) + 4}px`
  el.scrollIntoView({ block: 'start' })
}
