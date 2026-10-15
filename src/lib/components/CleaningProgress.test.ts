import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CleaningProgress from './CleaningProgress.svelte';

describe('CleaningProgress', () => {
  it('weighs progress by size and says what is left', () => {
    render(CleaningProgress, {
      progress: { completedItems: 1, totalItems: 3, completedBytes: 15_000_000_000, totalBytes: 20_000_000_000, currentLabel: 'Imagens sem contêiner' },
      startedAt: Date.now(),
    });

    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '75');
    expect(screen.getByText(/1 de 3 itens prontos. Faltam 5,0 GB em 2 itens./)).toBeInTheDocument();
    expect(screen.getByText('Imagens sem contêiner')).toBeInTheDocument();
  });

  it('starts at zero before the first update', () => {
    render(CleaningProgress, { progress: null, startedAt: Date.now() });

    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '0');
    expect(screen.getByText('Preparando a limpeza…')).toBeInTheDocument();
  });
});
