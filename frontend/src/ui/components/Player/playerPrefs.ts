import { createSignal } from 'solid-js'

const MUTED_KEY = 'ech-board-video-muted'
const VOLUME_KEY = 'ech-board-video-volume'
const PIP_WIDTH_KEY = 'ech-board-pip-width'
const AUTO_PIP_KEY = 'ech-board-auto-pip'
const STANDARD_PLAYER_KEY = 'ech-board-standard-player'

export const PIP_MIN_WIDTH = 240
export const PIP_MAX_WIDTH = 640
export const PIP_DEFAULT_WIDTH = 320

function load(): { muted: boolean; vol: number } {
  let muted = true
  let vol = 1
  try {
    const m = localStorage.getItem(MUTED_KEY)
    if (m != null) muted = m === 'true'
    const v = localStorage.getItem(VOLUME_KEY)
    if (v != null) {
      const n = Number(v)
      if (Number.isFinite(n)) vol = Math.min(Math.max(n, 0), 1)
    }
  } catch {
    return { muted: true, vol: 1 }
  }
  return { muted, vol }
}

function save(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    return
  }
}

function loadAutoPip(): boolean {
  try {
    const value = localStorage.getItem(AUTO_PIP_KEY)
    return value === null ? true : value === 'true'
  } catch {
    return true
  }
}

function loadStandardPlayer(): boolean {
  try {
    return localStorage.getItem(STANDARD_PLAYER_KEY) === 'true'
  } catch {
    return false
  }
}

function loadPipWidth(): number {
  try {
    const n = Number(localStorage.getItem(PIP_WIDTH_KEY))
    if (Number.isFinite(n) && n > 0) return Math.min(Math.max(n, PIP_MIN_WIDTH), PIP_MAX_WIDTH)
  } catch {
    return PIP_DEFAULT_WIDTH
  }
  return PIP_DEFAULT_WIDTH
}

const initial = load()
const [muted, setMutedSignal] = createSignal(initial.muted)
const [volume, setVolumeSignal] = createSignal(initial.vol)
const [pipWidth, setPipWidthSignal] = createSignal(loadPipWidth())
const [autoPip, setAutoPipSignal] = createSignal(loadAutoPip())
const [standardPlayer, setStandardPlayerSignal] = createSignal(loadStandardPlayer())

export const playerPrefs = {
  muted,
  volume,
  pipWidth,
  autoPip,
  standardPlayer,
  setStandardPlayer(v: boolean) {
    setStandardPlayerSignal(v)
    save(STANDARD_PLAYER_KEY, String(v))
  },
  setAutoPip(v: boolean) {
    setAutoPipSignal(v)
    save(AUTO_PIP_KEY, String(v))
  },
  setPipWidth(v: number) {
    const next = Math.min(Math.max(v, PIP_MIN_WIDTH), PIP_MAX_WIDTH)
    setPipWidthSignal(next)
    save(PIP_WIDTH_KEY, String(Math.round(next)))
  },
  setMuted(v: boolean) {
    setMutedSignal(v)
    save(MUTED_KEY, String(v))
  },
  setVolume(v: number) {
    const next = Math.min(Math.max(v, 0), 1)
    setVolumeSignal(next)
    save(VOLUME_KEY, String(next))
  },
}
