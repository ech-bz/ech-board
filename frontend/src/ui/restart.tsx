import { createContext, useContext, type JSX } from 'solid-js'

const RestartContext = createContext<() => void>(() => undefined)

export function RestartProvider(props: { restart: () => void; children: JSX.Element }) {
  return <RestartContext.Provider value={props.restart}>{props.children}</RestartContext.Provider>
}

export function useRestart(): () => void {
  return useContext(RestartContext)
}
