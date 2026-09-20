import type { MessageKey } from './i18n/ru'

export function settingLabel(name: string, t: (key: MessageKey) => string): string {
  if (name === 'Master Key') return t('ui.masterKey')
  return t('ui.appearance')
}
