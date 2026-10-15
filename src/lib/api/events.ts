import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CleanProgress } from './types';

export function onCleanProgress(handler: (progress: CleanProgress) => void): Promise<UnlistenFn> {
  return listen<CleanProgress>('clean:progress', (event) => handler(event.payload));
}
