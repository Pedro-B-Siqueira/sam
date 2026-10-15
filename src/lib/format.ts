const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const;
// Decimal units, like Finder and the macOS storage settings, so the numbers match what the user sees there.
const UNIT_STEP = 1000;

const oneDecimal = new Intl.NumberFormat('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const wholeNumber = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 0 });

export function formatBytes(bytes: number): string {
  let value = bytes;
  let unitIndex = 0;
  while (value >= UNIT_STEP && unitIndex < UNITS.length - 1) {
    value /= UNIT_STEP;
    unitIndex += 1;
  }
  const formatter = unitIndex === 0 || value >= 100 ? wholeNumber : oneDecimal;
  return `${formatter.format(value)} ${UNITS[unitIndex]}`;
}

export function pluralize(count: number, singular: string, plural: string): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

/** `/Users/me/field` → `~/field`, as macOS users read paths. */
export function shortenHome(path: string, homeDir: string): string {
  if (path === homeDir) return '~';
  return path.startsWith(`${homeDir}/`) ? `~${path.slice(homeDir.length)}` : path;
}

/** ['a', 'b', 'c'] → 'a, b e c'. */
export function joinWithAnd(parts: readonly string[]): string {
  if (parts.length <= 1) return parts.join('');
  return `${parts.slice(0, -1).join(', ')} e ${parts.at(-1) ?? ''}`;
}

/** 42 s → "0:42", 3 min 5 s → "3:05". */
export function formatElapsed(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  return `${minutes}:${String(totalSeconds % 60).padStart(2, '0')}`;
}
