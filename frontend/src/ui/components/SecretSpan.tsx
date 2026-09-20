import { createSignal, Show, type JSX } from 'solid-js'
import { cva } from 'class-variance-authority'
import { toHex, derivePostKeypair } from '../../core/intent/crypto'
import { tryDecryptSecret } from '../../core/intent/secret'
import { matchesAuthor } from '../../core/intent/roles'
import { fetchPostView } from '../../core/api/relay'
import { PostView as PostViewBcs } from '../../core/bcs/relay'
import { PostProjection } from '../../core/bcs/types'
import { useI18n } from '../i18n'
import { reportError } from '../errors'

const secretSpanVariants = cva('cursor-pointer font-mono text-accent')

export function SecretSpan(props: {
  dataNonce: Uint8Array
  dataCt: Uint8Array
  encryptedKeys: { nonce: Uint8Array; ct: Uint8Array; tag: Uint8Array }[]
  senderTweak: Uint8Array
  senderPub: Uint8Array
  masterSecret: Uint8Array
  forumId: Uint8Array
  replyTweaks: Uint8Array[]
  missingRefs: Uint8Array[]
  relayUrl: string
  renderNested: (bytes: Uint8Array) => JSX.Element
}) {
  const { t } = useI18n()
  const [revealed, setRevealed] = createSignal<JSX.Element>(null)

  const handleClick = async () => {
    if (revealed()) return
    const tweaks = [props.senderTweak, ...props.replyTweaks]
    for (const tweak of tweaks) {
      try {
        const { secretKey } = derivePostKeypair(props.masterSecret, props.forumId, tweak)
        const result = await tryDecryptSecret(props.dataNonce, props.dataCt, props.encryptedKeys, secretKey, props.senderPub)
        if (result !== null) {
          setRevealed(() => props.renderNested(result))
          return
        }
      } catch {
        continue
      }
    }
    if (props.relayUrl) {
      const seen = new Set<string>()
      for (const uid of props.missingRefs) {
        const uidHex = toHex(uid)
        if (seen.has(uidHex)) continue
        seen.add(uidHex)
        try {
          const buf = await fetchPostView(props.relayUrl, uidHex)
          const view = PostViewBcs.parse(new Uint8Array(buf))
          const pp = new PostProjection(view.post.projection)
          if (!matchesAuthor(props.masterSecret, props.forumId, pp.sender().pk, pp.sender().tweak)) continue
          const { secretKey } = derivePostKeypair(props.masterSecret, props.forumId, pp.sender().tweak)
          const result = await tryDecryptSecret(props.dataNonce, props.dataCt, props.encryptedKeys, secretKey, props.senderPub)
          if (result !== null) {
            setRevealed(() => props.renderNested(result))
            return
          }
        } catch {
          continue
        }
      }
    }
    reportError(t('post.secretDecryptFailed'))
  }

  return (
    <Show when={revealed()} fallback={<span class={secretSpanVariants()} onClick={() => void handleClick()}>{`[${t('post.secret')}]`}</span>}>
      {(content) => content()}
    </Show>
  )
}
