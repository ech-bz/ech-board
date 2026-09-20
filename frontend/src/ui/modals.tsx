import { createContext, createSignal, Show, useContext, type JSX } from 'solid-js'
import { useMod } from './mod'
import { useStore } from './store'
import { canBoardCore, canBoardSettings, type BanScope } from '../core/intent/capabilities'
import { getForumModerators } from '../core/api/cache'
import { BanModal } from './components/BanModal'
import { BansModal } from './components/BansModal'
import { BoardSettingsModal } from './components/BoardSettingsModal'
import { LogsModal, type LogsTab } from './components/LogsModal/LogsModal'
import { ModeratorsModal } from './components/ModeratorsModal'
import { ThreadTopicModal } from './components/ThreadTopicModal'
import type { EntityObject } from './components/debug/EntityDebugBody'
import type { PostObject, ModeratorsData } from '../core/bcs/types'
import type { RoleKind, RoleOption } from '../core/intent/roles'
import type { EntityNs } from '../core/intent/upgrade'

interface BanTarget {
  posts: PostObject[]
  threadUid: string
  deletePosts?: boolean
}

export interface ModalsApi {
  openThreadTopic: (threadUid: string) => void
  openThreadModerators: () => void
  openBoardModerators: () => void
  openForumModerators: () => void
  openBoardSettings: () => void
  openBan: (posts: PostObject[], threadUid: string, deletePosts?: boolean) => void
  openBanUid: () => void
  openBans: (title: string, uid: string) => void
  openLogs: (tab: LogsTab, object?: EntityObject) => void
}

const ModalsCtx = createContext<ModalsApi>()

export function ModalsProvider(props: { roles: RoleOption[]; relayUrl: string; children: JSX.Element }) {
  const store = useStore()
  const mod = useMod()
  const [topic, setTopic] = createSignal<{ threadUid: string; topic: string } | null>(null)
  const [moderators, setModerators] = createSignal<ModeratorsData | null>(null)
  const [banTarget, setBanTarget] = createSignal<BanTarget | null>(null)
  const [banUidOpen, setBanUidOpen] = createSignal(false)
  const [bans, setBans] = createSignal<{ title: string; uid: string } | null>(null)
  const [boardSettings, setBoardSettings] = createSignal(false)
  const [logs, setLogs] = createSignal<{ tab: LogsTab; object: EntityObject | null } | null>(null)

  const roleKinds = (): RoleKind[] => props.roles.map((role) => role.kind)

  const api: ModalsApi = {
    openThreadTopic: (threadUid) => setTopic({ threadUid, topic: store.threadTopic(threadUid) }),
    openThreadModerators: () => setModerators(store.info()?.moderators ?? null),
    openBoardModerators: () => setModerators(store.info()?.moderators ?? null),
    openForumModerators: () => setModerators(getForumModerators(props.relayUrl)),
    openBoardSettings: () => setBoardSettings(true),
    openBan: (posts, threadUid, deletePosts) => setBanTarget({ posts, threadUid, deletePosts }),
    openBanUid: () => setBanUidOpen(true),
    openBans: (title, uid) => setBans({ title, uid }),
    openLogs: (tab, object) => setLogs({ tab, object: object ?? null }),
  }

  return (
    <ModalsCtx.Provider value={api}>
      {props.children}
      <Show when={topic()}>
        {(state) => (
          <ThreadTopicModal
            threadUid={state().threadUid}
            initial={state().topic}
            busy={mod.busy()}
            onClose={() => setTopic(null)}
            onSave={async (threadUid, text) => {
              await mod.runThreadEditTopic(threadUid, text)
              setTopic(null)
            }}
          />
        )}
      </Show>
      <Show when={moderators()}>
        {(value) => <ModeratorsModal moderators={value()} onClose={() => setModerators(null)} />}
      </Show>
      <Show when={banTarget() || banUidOpen()}>
        <BanModal
          mode={banUidOpen() ? 'uid' : 'post'}
          roleKinds={roleKinds()}
          busy={mod.busy()}
          onClose={() => {
            setBanTarget(null)
            setBanUidOpen(false)
          }}
          onBan={async (scope, mask, durationMs, reason, uidHex) => {
            if (uidHex != null) {
              const threadUid = store.threadUid()
              await mod.runBanUid(uidHex, scope === 'thread' ? (threadUid ?? '') : '', scope as BanScope, mask, durationMs, reason)
              setBanUidOpen(false)
              return
            }
            const target = banTarget()
            if (!target) return
            if (target.posts.length === 1) await mod.runBan(target.posts[0], target.threadUid, scope, mask, durationMs, reason)
            else await mod.runBatchBan(target.posts, target.threadUid, scope, mask, durationMs, reason)
            if (target.deletePosts) {
              if (target.posts.length === 1) await mod.runPostDelete(target.posts[0], 'delete', target.threadUid)
              else await mod.runBatchPostDelete(target.posts, target.threadUid)
            }
            setBanTarget(null)
          }}
        />
      </Show>
      <Show when={bans()}>
        {(value) => <BansModal title={value().title} relayUrl={props.relayUrl} uid={value().uid} onClose={() => setBans(null)} />}
      </Show>
      <Show when={logs()}>
        {(state) => (
          <LogsModal
            tab={state().tab}
            relayUrl={props.relayUrl}
            boardUid={store.info()?.boardUid}
            object={state().object}
            onUpgrade={(object) => void mod.runUpgrade(state().tab as EntityNs, object as PostObject)}
            onClose={() => setLogs(null)}
          />
        )}
      </Show>
      <Show when={boardSettings() && store.boardObject() && store.info()}>
        <BoardSettingsModal
          onClose={() => setBoardSettings(false)}
          board={store.boardObject()!}
          description={store.boardInfo()?.description ?? ''}
          moderators={store.boardModerators()}
          relayUrl={store.info()!.relayUrl}
          boardUid={store.info()!.boardUid}
          canCore={canBoardCore(roleKinds())}
          canSoft={canBoardSettings(roleKinds())}
          mod={{
            busy: mod.busy(),
            runBoardSetDescription: (text) => mod.runBoardSetDescription(text),
            runBoardSetMaxMedia: (v) => mod.runBoardSetMaxMedia(v),
            runBoardSetBumpLimit: (v) => mod.runBoardSetBumpLimit(v),
            runBoardSetIgnoreForumBans: (v) => mod.runBoardSetIgnoreForumBans(v),
            runBoardSetReactions: (vals) => mod.runBoardSetReactions(vals),
            runBoardSetPinned: (pinned) => mod.runBoardSetPinned(pinned),
            runBoardAddModerator: (addr) => mod.runBoardAddModerator(addr),
            runBoardDelModerator: (addr) => mod.runBoardDelModerator(addr),
          }}
        />
      </Show>
    </ModalsCtx.Provider>
  )
}

export function useModals(): ModalsApi {
  const value = useContext(ModalsCtx)
  if (!value) throw new Error('useModals outside provider')
  return value
}
