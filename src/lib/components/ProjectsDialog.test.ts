import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Settings, SettingsView } from '../api/types';
import ProjectsDialog from './ProjectsDialog.svelte';

function view(settings: Settings): SettingsView {
  return {
    settings,
    homeDir: '/Users/me',
    detectedRoots: [{ path: '/Users/me/field', location: '~/field', markers: ['eyeofsauron-2', 'mordor-2'] }],
    effectiveRoots: ['~/field'],
    gitAvailable: true,
  };
}

function renderDialog(settings: Settings) {
  const onSave = vi.fn();
  render(ProjectsDialog, { view: view(settings), saving: false, error: null, onCancel: vi.fn(), onSave });
  return { onSave };
}

describe('ProjectsDialog', () => {
  it('shows what was detected and saves automatic mode as an empty list', async () => {
    const { onSave } = renderDialog({ repoRoots: [], idleDays: 30 });

    expect(screen.getByText('~/field')).toBeInTheDocument();
    expect(screen.getByText('tem eyeofsauron-2 e mordor-2')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Salvar e medir' }));

    expect(onSave).toHaveBeenCalledWith({ repoRoots: [], idleDays: 30 });
  });

  it('starts the manual list from the detected folders', async () => {
    const { onSave } = renderDialog({ repoRoots: [], idleDays: 30 });

    await userEvent.click(screen.getByRole('radio', { name: /Escolher as pastas/ }));
    await userEvent.selectOptions(screen.getByRole('combobox'), '90');
    await userEvent.click(screen.getByRole('button', { name: 'Salvar e medir' }));

    expect(onSave).toHaveBeenCalledWith({ repoRoots: ['/Users/me/field'], idleDays: 90 });
  });

  it('will not save a manual list with no folders', async () => {
    renderDialog({ repoRoots: ['/Users/me/projetos'], idleDays: 30 });

    expect(screen.getByText('~/projetos')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Remover' }));

    expect(screen.getByText('Adicione ao menos uma pasta.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Salvar e medir' })).toBeDisabled();
  });

  it('warns when git is missing', () => {
    render(ProjectsDialog, {
      view: { ...view({ repoRoots: [], idleDays: 30 }), gitAvailable: false },
      saving: false,
      error: null,
      onCancel: vi.fn(),
      onSave: vi.fn(),
    });

    expect(screen.getByRole('alert')).toHaveTextContent('xcode-select --install');
  });
});
