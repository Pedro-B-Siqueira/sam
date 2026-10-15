<script lang="ts">
  import { untrack } from 'svelte';
  import { pickFolder } from '../api/commands';
  import { describeFailure, type FailureCause } from '../api/logger';
  import type { Settings, SettingsView } from '../api/types';
  import { joinWithAnd, shortenHome } from '../format';
  import Dialog from './Dialog.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    view: SettingsView;
    saving: boolean;
    error: string | null;
    onCancel: () => void;
    onSave: (settings: Settings) => void;
  }

  let { view, saving, error, onCancel, onSave }: Props = $props();

  const IDLE_DAY_OPTIONS = [7, 14, 30, 60, 90, 180];

  // The dialog opens fresh each time; editing starts from the saved settings.
  const saved = untrack(() => view.settings);
  const detectedPaths = untrack(() => view.detectedRoots.map((root) => root.path));
  let mode = $state<'auto' | 'manual'>(saved.repoRoots.length > 0 ? 'manual' : 'auto');
  let chosen = $state<string[]>(saved.repoRoots.length > 0 ? [...saved.repoRoots] : detectedPaths);
  let idleDays = $state(saved.idleDays);
  let pickError = $state<string | null>(null);

  const idleOptions = $derived(IDLE_DAY_OPTIONS.includes(idleDays) ? IDLE_DAY_OPTIONS : [...IDLE_DAY_OPTIONS, idleDays].sort((a, b) => a - b));
  const canSave = $derived(!saving && (mode === 'auto' || chosen.length > 0));

  async function addFolder(): Promise<void> {
    pickError = null;
    const picked = await pickFolder('Escolha a pasta onde ficam seus repositórios').catch((cause: FailureCause) => {
      pickError = `Não deu para abrir a escolha de pasta: ${describeFailure(cause)}`;
      return null;
    });
    if (picked && !chosen.includes(picked)) chosen = [...chosen, picked];
  }

  function save(): void {
    onSave({ repoRoots: mode === 'auto' ? [] : chosen, idleDays });
  }
</script>

<Dialog title="Pastas de projetos" width={600} onClose={() => !saving && onCancel()}>
  <p class="intro">
    O Sam procura node_modules e saídas de build dentro dos repositórios git destas pastas. Ele nunca mexe no código, só em pastas
    que o git ignora.
  </p>

  {#if !view.gitAvailable}
    <p class="warning" role="alert">
      O git não foi encontrado neste Mac, então os projetos não podem ser analisados. Instale as ferramentas de desenvolvedor com
      xcode-select --install e abra o Sam de novo.
    </p>
  {/if}

  <fieldset>
    <legend class="visually-hidden">Como encontrar as pastas</legend>

    <label class="option">
      <input type="radio" name="mode" value="auto" bind:group={mode} />
      <span class="option-text">
        <span class="option-title">Encontrar automaticamente</span>
        <span class="option-hint">Procura, até dois níveis abaixo da sua pasta pessoal, onde estão eyeofsauron, eyeofsauron-2 ou mordor-2.</span>
      </span>
    </label>
    {#if mode === 'auto'}
      <div class="nested">
        {#if view.detectedRoots.length === 0}
          <p class="empty">Nada encontrado. Escolha as pastas manualmente.</p>
        {:else}
          <ul class="folders">
            {#each view.detectedRoots as root (root.path)}
              <li>
                <Icon name="folder" />
                <span class="folder-path">{root.location}</span>
                <span class="folder-hint">tem {joinWithAnd(root.markers)}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <label class="option">
      <input type="radio" name="mode" value="manual" bind:group={mode} />
      <span class="option-text">
        <span class="option-title">Escolher as pastas</span>
        <span class="option-hint">Para quem guarda os repositórios em outro lugar ou em mais de uma pasta.</span>
      </span>
    </label>
    {#if mode === 'manual'}
      <div class="nested">
        {#if chosen.length > 0}
          <ul class="folders">
            {#each chosen as folder (folder)}
              <li>
                <Icon name="folder" />
                <span class="folder-path">{shortenHome(folder, view.homeDir)}</span>
                <button type="button" class="button-quiet small" onclick={() => (chosen = chosen.filter((entry) => entry !== folder))}>
                  Remover
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">Adicione ao menos uma pasta.</p>
        {/if}
        <button type="button" class="button-tool" onclick={addFolder}><Icon name="plus" /> Adicionar pasta</button>
        {#if pickError}<p class="warning">{pickError}</p>{/if}
      </div>
    {/if}
  </fieldset>

  <label class="idle">
    <span class="option-title">Considerar parado um projeto sem commit há</span>
    <select bind:value={idleDays}>
      {#each idleOptions as days (days)}
        <option value={days}>{days} dias</option>
      {/each}
    </select>
    <span class="option-hint">Os node_modules dos projetos ativos nunca aparecem para limpar.</span>
  </label>

  {#if error}
    <p class="warning" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <button type="button" class="button-quiet" disabled={saving} onclick={onCancel}>Cancelar</button>
    <button type="button" class="button-primary" disabled={!canSave} onclick={save}>{saving ? 'Salvando…' : 'Salvar e medir'}</button>
  {/snippet}
</Dialog>

<style>
  .intro {
    max-width: 62ch;
    margin: 0 0 16px;
    color: var(--ink-soft);
  }

  fieldset {
    display: grid;
    gap: 10px;
    margin: 0 0 18px;
    padding: 0;
    border: 0;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .option {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: start;
    cursor: pointer;
  }

  .option input {
    margin-top: 3px;
    accent-color: var(--moss);
  }

  .option-text {
    display: grid;
  }

  .option-title {
    font-weight: 650;
  }

  .option-hint {
    color: var(--ink-soft);
    font-size: 12.5px;
  }

  .nested {
    display: grid;
    justify-items: start;
    gap: 10px;
    margin-left: 26px;
    padding: 12px 14px;
    border-radius: var(--radius-control);
    background: var(--paper);
  }

  .folders {
    display: grid;
    gap: 6px;
    width: 100%;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .folders li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    color: var(--moss);
  }

  .folder-path {
    color: var(--ink);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .folder-hint {
    color: var(--ink-soft);
    font-size: 12.5px;
  }

  .small {
    height: 26px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .empty {
    margin: 0;
    color: var(--ink-soft);
  }

  .idle {
    display: grid;
    justify-items: start;
    gap: 6px;
  }

  select {
    height: 32px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    border-radius: var(--radius-control);
    background: var(--paper);
    color: var(--ink);
    font: inherit;
  }

  .warning {
    margin: 12px 0 0;
    padding: 10px 12px;
    border-radius: var(--radius-control);
    background: color-mix(in srgb, var(--caution) 14%, transparent);
    color: var(--caution);
  }
</style>
