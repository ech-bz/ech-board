import { createContext, useContext, type Accessor } from 'solid-js'

export const PotatoHeadContext = createContext<Accessor<HTMLElement | undefined>>(() => undefined)

export function usePotatoHead(): Accessor<HTMLElement | undefined> {
  return useContext(PotatoHeadContext)
}
