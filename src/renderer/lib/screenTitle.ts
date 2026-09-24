import type { Screen } from '../../shared/types'

export function screenTitle(screen: Screen): string {
  switch (screen) {
    case 'transcribe':
    case 'transcribe-done':
      return 'Transcribe'
    case 'find':
    case 'review':
      return 'Find best parts'
    case 'export':
      return 'Export'
    case 'reframe':
      return 'Reframe'
    case 'captions':
      return 'Captions'
    default:
      return ''
  }
}

export function isPlayerScreen(screen: Screen): boolean {
  return screen === 'review' || screen === 'reframe' || screen === 'captions'
}
