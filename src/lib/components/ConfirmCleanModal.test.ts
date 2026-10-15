import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { SelectedGroup } from '../selection';
import { makeCategory, makeItem } from '../test-fixtures';
import ConfirmCleanModal from './ConfirmCleanModal.svelte';

function group(id: string, reversibility: 'regenerable' | 'irreversible', sizeBytes: number): SelectedGroup {
  const item = makeItem(`${id}-item`, sizeBytes);
  return { category: makeCategory(id, 'safe', reversibility), items: [item], totalBytes: sizeBytes };
}

describe('ConfirmCleanModal', () => {
  it('shows the total and, for each group, what happens and how to undo', () => {
    render(ConfirmCleanModal, {
      groups: [group('npm', 'regenerable', 2_000_000_000), group('brew', 'regenerable', 1_000_000_000)],
      colorOf: () => 'var(--plot-1)',
      cleaning: false,
      progress: null,
      startedAt: 0,
      onCancel: vi.fn(),
      onConfirm: vi.fn(),
    });

    expect(screen.getByText('3,0 GB')).toBeInTheDocument();
    expect(screen.getByText(/Efeito de limpar npm/)).toBeInTheDocument();
    expect(screen.getByText(/Detalhe de reversão de brew/)).toBeInTheDocument();
    expect(screen.getByText(/Recomendação para npm/)).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('warns when something cannot be recovered', () => {
    render(ConfirmCleanModal, {
      groups: [group('volumes', 'irreversible', 500)],
      colorOf: () => 'var(--plot-1)',
      cleaning: false,
      progress: null,
      startedAt: 0,
      onCancel: vi.fn(),
      onConfirm: vi.fn(),
    });

    expect(screen.getByRole('alert')).toHaveTextContent('Categoria volumes');
  });

  it('confirms only through the clean button', async () => {
    const onConfirm = vi.fn();
    render(ConfirmCleanModal, { groups: [group('npm', 'regenerable', 1_500)], colorOf: () => 'var(--plot-1)', cleaning: false, progress: null, startedAt: 0, onCancel: vi.fn(), onConfirm });

    await userEvent.click(screen.getByRole('button', { name: 'Limpar 1,5 KB' }));

    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('locks the buttons while cleaning', () => {
    render(ConfirmCleanModal, { groups: [group('npm', 'regenerable', 1_500)], colorOf: () => 'var(--plot-1)', cleaning: true, progress: null, startedAt: 0, onCancel: vi.fn(), onConfirm: vi.fn() });

    expect(screen.getByRole('button', { name: 'Limpando…' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Voltar' })).not.toBeInTheDocument();
  });

  it('turns into the progress view while cleaning', () => {
    render(ConfirmCleanModal, {
      groups: [group('npm', 'regenerable', 2_000)],
      colorOf: () => 'var(--plot-1)',
      cleaning: true,
      progress: { completedItems: 0, totalItems: 1, completedBytes: 0, totalBytes: 2_000, currentLabel: 'Item npm-item' },
      startedAt: Date.now(),
      onCancel: vi.fn(),
      onConfirm: vi.fn(),
    });

    expect(screen.getByRole('heading', { name: 'Limpando' })).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.queryByText(/Efeito de limpar npm/)).not.toBeInTheDocument();
  });
});
