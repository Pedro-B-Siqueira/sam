import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CleanOutcome } from '../api/types';
import CleanResults from './CleanResults.svelte';

const done: CleanOutcome = { categoryId: 'large-downloads', itemId: 'a', label: 'Docker.dmg', sizeBytes: 600_000_000, status: 'done', output: null };

describe('CleanResults', () => {
  it('reminds that trashed items still take space and offers to open the Trash', () => {
    render(CleanResults, { outcomes: [done], diskBefore: null, diskAfter: null, trashedBytes: 600_000_000, reportStatus: null, onSaveReport: vi.fn(), onClose: vi.fn() });

    expect(screen.getByRole('note')).toHaveTextContent('600 MB foram para a Lixeira e ainda ocupam o disco');
    expect(screen.getByRole('button', { name: 'Abrir a Lixeira' })).toBeInTheDocument();
  });

  it('says nothing about the Trash when nothing went there', () => {
    render(CleanResults, { outcomes: [done], diskBefore: null, diskAfter: null, trashedBytes: 0, reportStatus: null, onSaveReport: vi.fn(), onClose: vi.fn() });

    expect(screen.queryByRole('note')).not.toBeInTheDocument();
  });

  it('lists what failed', () => {
    const failed: CleanOutcome = { categoryId: 'npm-cache', itemId: 'b', label: 'Pacotes', sizeBytes: 1, status: 'failed', message: 'permissão negada' };
    render(CleanResults, { outcomes: [done, failed], diskBefore: null, diskAfter: null, trashedBytes: 0, reportStatus: null, onSaveReport: vi.fn(), onClose: vi.fn() });

    expect(screen.getByText('permissão negada', { exact: false })).toBeInTheDocument();
  });

  it('offers to save the report and shows where it went', async () => {
    const onSaveReport = vi.fn();
    render(CleanResults, {
      outcomes: [done],
      diskBefore: null,
      diskAfter: null,
      trashedBytes: 0,
      reportStatus: 'Relatório salvo em ~/Downloads/sam-limpeza.md.',
      onSaveReport,
      onClose: vi.fn(),
    });

    await userEvent.click(screen.getByRole('button', { name: 'Salvar relatório' }));

    expect(onSaveReport).toHaveBeenCalledOnce();
    expect(screen.getByRole('status')).toHaveTextContent('Relatório salvo em ~/Downloads/sam-limpeza.md.');
  });
});
