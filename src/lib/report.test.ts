import { describe, expect, it } from 'vitest';
import type { CleanOutcome } from './api/types';
import { buildReport, reportFileName } from './report';
import { makeCategory } from './test-fixtures';

const npm = { ...makeCategory('npm-cache'), title: 'Cache do npm' };
const downloads = { ...makeCategory('large-downloads', 'review', 'trash'), title: 'Downloads grandes e antigos' };
const finishedAt = new Date(2026, 9, 15, 14, 32);

const outcomes: CleanOutcome[] = [
  { categoryId: 'npm-cache', itemId: 'a', label: 'Pacotes baixados pelo npm', sizeBytes: 13_500_000_000, status: 'done', output: null },
  { categoryId: 'large-downloads', itemId: 'b', label: 'Docker.dmg', sizeBytes: 600_000_000, status: 'done', output: null },
  { categoryId: 'npm-cache', itemId: 'c', label: 'Pacotes rodados com npx', sizeBytes: 1_000_000_000, status: 'failed', message: 'permissão negada' },
];

describe('buildReport', () => {
  const report = buildReport({
    outcomes,
    categories: [npm, downloads],
    diskBefore: { totalBytes: 245_000_000_000, availableBytes: 17_400_000_000, usedBytes: 227_600_000_000 },
    diskAfter: { totalBytes: 245_000_000_000, availableBytes: 30_900_000_000, usedBytes: 214_100_000_000 },
    trashedBytes: 600_000_000,
    finishedAt,
  });

  it('opens with when it happened and the free space before and after', () => {
    expect(report).toContain('# Limpeza feita com o Sam');
    expect(report).toContain('15/10/2026');
    expect(report).toContain('| Antes | 17,4 GB |');
    expect(report).toContain('| Depois | 30,9 GB |');
  });

  it('sums only what was really cleaned and reminds about the Trash', () => {
    expect(report).toContain('O Sam limpou 14,1 GB em 2 itens.');
    expect(report).toContain('600 MB foram para a Lixeira');
  });

  it('groups items by category and lists failures apart', () => {
    expect(report).toContain('### Cache do npm\n\n- Pacotes baixados pelo npm: 13,5 GB');
    expect(report).toContain('### Downloads grandes e antigos\n\n- Docker.dmg: 600 MB');
    expect(report).toContain('## O que não deu para limpar\n\n- Pacotes rodados com npx: permissão negada');
  });
});

describe('reportFileName', () => {
  it('is sortable and carries the minute', () => {
    expect(reportFileName(finishedAt)).toBe('sam-limpeza-2026-10-15-1432.md');
  });
});
