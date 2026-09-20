export interface MoveAbortInfo {
  module: string
  code: number
  message: string
}

const MOVE_ABORT_MESSAGES: Record<number, string> = {
  1: 'Неверная подпись интента',
  2: 'Несовпадение цели интента',
  3: 'Несовпадение объекта интента',
  4: 'Несовпадение аргументов интента',
  5: 'Неверная подпись relay',
  6: 'Несовпадение индекса счётчика',
  7: 'Несовпадение значения счётчика',
  8: 'Некорректный слаг доски',
  9: 'Превышен лимит вложений',
  10: 'Пост требует вложение',
  11: 'Пустой пост',
  12: 'Доска закрыта',
  13: 'Тред закрыт',
  14: 'Нет прав',
  15: 'Несовпадение перекрёстной ссылки',
  16: 'Реакция не разрешена',
  17: 'Уже голосовал',
  18: 'Неподдерживаемая версия события',
  19: 'Entity устарела',
  20: 'Несовпадение вариантов голосования',
  21: 'Превышен лимит вариантов голосования',
  22: 'Вложение не найдено',
}

export function parseMoveAbort(e: unknown): MoveAbortInfo | null {
  const s = String(e)
  const moduleMatch = s.match(/Identifier\(\\?"?([A-Za-z_0-9]+)\\?"?\)/)
  const addrMatch = s.match(/address:\s*([0-9a-fA-F]{64})/)
  const codeMatch = s.match(/MoveAbort\([^]*?,\s*(\d+)\) in command/)
  if (!codeMatch) return null
  const code = parseInt(codeMatch[1], 10)
  const addr = addrMatch ? addrMatch[1].toLowerCase() : ''
  const isFramework = addr.length === 64 && addr.slice(0, 60) === '0'.repeat(60)
  return {
    module: moduleMatch ? moduleMatch[1] : 'unknown',
    code,
    message: isFramework ? `Ошибка ${code}` : (MOVE_ABORT_MESSAGES[code] ?? `Ошибка ${code}`),
  }
}
