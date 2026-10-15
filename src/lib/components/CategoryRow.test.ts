import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CategoryScan } from '../api/types';
import { makeCategory, makeItem, makeScan } from '../test-fixtures';
import CategoryRow from './CategoryRow.svelte';

const category = makeCategory('npm-cache');

function renderCard(scan: CategoryScan, selected: ReadonlySet<string> = new Set()) {
  const onToggleItem = vi.fn();
  const onSetAll = vi.fn();
  render(CategoryRow, { category, color: 'var(--plot-1)', scanState: { status: 'done', scan }, selected, disabled: false, onToggleItem, onSetAll });
  return { onToggleItem, onSetAll };
}

describe('CategoryRow', () => {
  it('shows the explanation and the items only when asked', async () => {
    renderCard(makeScan('npm-cache', [makeItem('a', 2_000_000)]));
    expect(screen.queryByText('O que é npm-cache')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /Entender antes de limpar/ }));

    expect(screen.getByText('O que é npm-cache')).toBeInTheDocument();
    expect(screen.getByText('Efeito de limpar npm-cache')).toBeInTheDocument();
    expect(screen.getByText('Item a')).toBeInTheDocument();
  });

  it('toggles a single item', async () => {
    const { onToggleItem } = renderCard(makeScan('npm-cache', [makeItem('a', 10), makeItem('b', 20)]), new Set(['a']));
    await userEvent.click(screen.getByRole('button', { name: /Entender antes de limpar/ }));

    await userEvent.click(screen.getByRole('checkbox', { name: /Item b/ }));

    expect(onToggleItem).toHaveBeenCalledWith('b');
  });

  it('selects everything from a partial selection', async () => {
    const { onSetAll } = renderCard(makeScan('npm-cache', [makeItem('a', 10), makeItem('b', 20)]), new Set(['a']));

    await userEvent.click(screen.getByRole('checkbox', { name: /Marcar tudo/ }));

    expect(onSetAll).toHaveBeenCalledWith(true);
  });

  it('explains why there is nothing to clean', () => {
    renderCard({ categoryId: 'homebrew', items: [], totalBytes: 0, footnote: null, issue: { kind: 'toolMissing', tool: 'Homebrew' } });

    expect(screen.getByText(/Homebrew não está instalado/)).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: /Marcar tudo/ })).toBeDisabled();
  });

  it('tells the user how to grant access when macOS blocks a folder', () => {
    renderCard({ categoryId: 'app-caches', items: [], totalBytes: 0, footnote: null, issue: { kind: 'noAccess', location: '~/Library/Caches' } });

    expect(screen.getByText(/Acesso Total ao Disco/)).toBeInTheDocument();
  });

  it('says what the scan left out on purpose', () => {
    renderCard({ ...makeScan('project-dependencies', [makeItem('a', 10)]), footnote: '3 projetos com commit nos últimos 30 dias ficaram de fora.' });

    expect(screen.getByText('3 projetos com commit nos últimos 30 dias ficaram de fora.')).toBeInTheDocument();
  });

  it('points to the projects settings when no folder was found', () => {
    renderCard({ categoryId: 'build-outputs', items: [], totalBytes: 0, footnote: null, issue: { kind: 'noProjectFolders' } });

    expect(screen.getByText(/Escolha onde ficam seus repositórios em Projetos/)).toBeInTheDocument();
  });

  it('asks to open the Docker engine when it is off', () => {
    renderCard({ categoryId: 'docker-images', items: [], totalBytes: 0, footnote: null, issue: { kind: 'notRunning', engine: 'OrbStack' } });

    expect(screen.getByText(/O OrbStack está desligado. Abra o OrbStack/)).toBeInTheDocument();
  });
});
