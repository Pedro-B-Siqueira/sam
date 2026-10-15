import type { CategoryInfo, CleanOutcome, DiskUsage } from './api/types';
import { formatBytes, pluralize } from './format';

export interface ReportInput {
  outcomes: readonly CleanOutcome[];
  categories: readonly CategoryInfo[];
  diskBefore: DiskUsage | null;
  diskAfter: DiskUsage | null;
  trashedBytes: number;
  finishedAt: Date;
}

const dateTime = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'short' });

/** A Markdown record of one cleanup, written for a person to read later or share. */
export function buildReport(input: ReportInput): string {
  const done = input.outcomes.filter((outcome) => outcome.status === 'done');
  const failed = input.outcomes.flatMap((outcome) => (outcome.status === 'failed' ? [outcome] : []));
  const cleanedBytes = done.reduce((total, outcome) => total + outcome.sizeBytes, 0);
  const lines = [
    '# Limpeza feita com o Sam',
    '',
    `${dateTime.format(input.finishedAt)}.`,
    '',
    ...diskSection(input.diskBefore, input.diskAfter),
    `O Sam limpou ${formatBytes(cleanedBytes)} em ${pluralize(done.length, 'item', 'itens')}.`,
  ];
  if (input.trashedBytes > 0) {
    lines.push('', `${formatBytes(input.trashedBytes)} foram para a Lixeira e só liberam espaço quando ela for esvaziada.`);
  }
  lines.push(...cleanedSection(done, input.categories), ...failedSection(failed));
  return `${lines.join('\n')}\n`;
}

/** `sam-limpeza-2026-10-15-1432.md`, sortable and unique per minute. */
export function reportFileName(finishedAt: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0');
  const date = `${finishedAt.getFullYear()}-${pad(finishedAt.getMonth() + 1)}-${pad(finishedAt.getDate())}`;
  return `sam-limpeza-${date}-${pad(finishedAt.getHours())}${pad(finishedAt.getMinutes())}.md`;
}

function diskSection(before: DiskUsage | null, after: DiskUsage | null): string[] {
  if (!before || !after) return [];
  return ['| | Espaço livre |', '|---|---|', `| Antes | ${formatBytes(before.availableBytes)} |`, `| Depois | ${formatBytes(after.availableBytes)} |`, ''];
}

function cleanedSection(done: readonly CleanOutcome[], categories: readonly CategoryInfo[]): string[] {
  if (done.length === 0) return [];
  const lines = ['', '## O que foi limpo'];
  for (const category of categories) {
    const cleaned = done.filter((outcome) => outcome.categoryId === category.id);
    if (cleaned.length === 0) continue;
    lines.push('', `### ${category.title}`, '');
    lines.push(...cleaned.map((outcome) => `- ${outcome.label}: ${formatBytes(outcome.sizeBytes)}`));
  }
  return lines;
}

function failedSection(failed: readonly Extract<CleanOutcome, { status: 'failed' }>[]): string[] {
  if (failed.length === 0) return [];
  return ['', '## O que não deu para limpar', '', ...failed.map((outcome) => `- ${outcome.label}: ${outcome.message}`)];
}
