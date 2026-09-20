import { createContext, useContext, onCleanup, type JSX } from 'solid-js'
import { createQuoteReply } from './quoteReply'

type Insert = (text: string) => void

interface Registered {
  token: object
  insert: Insert
  isVisible: () => boolean
}

interface Api {
  setInsert: (fn: Insert | null, token?: object, isVisible?: () => boolean) => void
  insert: (text: string) => void
  takeQuote: () => string
}

const Ctx = createContext<Api>({ setInsert: () => {}, insert: () => {}, takeQuote: () => '' })

export function ComposerBridgeProvider(props: { children: JSX.Element }) {
  let composers: Registered[] = []
  const { takeQuote } = createQuoteReply()

  const setInsert = (fn: Insert | null, token?: object, isVisible?: () => boolean) => {
    if (token == null) return
    if (fn) {
      composers = [...composers.filter((c) => c.token !== token), { token, insert: fn, isVisible: isVisible ?? (() => true) }]
    } else {
      composers = composers.filter((c) => c.token !== token)
    }
  }

  const insert = (text: string) => {
    composers.find((c) => c.isVisible())?.insert(text)
  }

  const api: Api = { setInsert, insert, takeQuote }
  onCleanup(() => {
    composers = []
  })
  return <Ctx.Provider value={api}>{props.children}</Ctx.Provider>
}

export function useComposerBridge(): Api {
  return useContext(Ctx)
}
