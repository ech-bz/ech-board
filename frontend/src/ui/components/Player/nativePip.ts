interface WebkitVideo extends HTMLVideoElement {
  webkitSupportsPresentationMode?: (mode: string) => boolean
  webkitSetPresentationMode?: (mode: string) => void
  webkitPresentationMode?: string
}

export function canUseNativePip(media: HTMLMediaElement): boolean {
  if (!(media instanceof HTMLVideoElement)) return false
  if (media.disablePictureInPicture) return false
  if (document.pictureInPictureEnabled) return true
  return !!(media as WebkitVideo).webkitSupportsPresentationMode?.('picture-in-picture')
}

export async function enterNativePip(media: HTMLVideoElement): Promise<boolean> {
  try {
    if (document.pictureInPictureEnabled) {
      await media.requestPictureInPicture()
      return true
    }
    const w = media as WebkitVideo
    if (w.webkitSupportsPresentationMode?.('picture-in-picture')) {
      w.webkitSetPresentationMode?.('picture-in-picture')
      return true
    }
  } catch {
    return false
  }
  return false
}

export function onNativePipLeave(media: HTMLMediaElement, callback: () => void): () => void {
  const onStd = () => callback()
  const onWebkit = () => {
    if ((media as WebkitVideo).webkitPresentationMode !== 'picture-in-picture') callback()
  }
  media.addEventListener('leavepictureinpicture', onStd)
  media.addEventListener('webkitpresentationmodechanged', onWebkit)
  return () => {
    media.removeEventListener('leavepictureinpicture', onStd)
    media.removeEventListener('webkitpresentationmodechanged', onWebkit)
  }
}

export function exitNativePip(): void {
  if (document.pictureInPictureElement) document.exitPictureInPicture().catch(() => undefined)
}
