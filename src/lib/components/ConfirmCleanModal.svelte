<script lang="ts">
  import type { CleanProgress } from '../api/types';
  import { formatBytes, pluralize } from '../format';
  import { REVERSIBILITY_LABEL } from '../labels';
  import type { SelectedGroup } from '../selection';
  import CleaningProgress from './CleaningProgress.svelte';
  import Dialog from './Dialog.svelte';

  interface Props {
    groups: SelectedGroup[];
    colorOf: (categoryId: string) => string;
    cleaning: boolean;
    progress: CleanProgress | null;
    startedAt: number;
    onCancel: () => void;
    onConfirm: () => void;
  }

  let { groups, colorOf, cleaning, progress, startedAt, onCancel, onConfirm }: Props = $props();

  const totalBytes = $derived(groups.reduce((total, group) => total + group.totalBytes, 0));
  const itemCount = $derived(groups.reduce((total, group) => total + group.items.length, 0));
  const trashBytes = $derived(
    groups.filter((group) => group.category.explanation.reversibility === 'trash').reduce((total, group) => total + group.totalBytes, 0),
  );
  const irreversibleTitles = $derived(
    groups.filter((group) => group.category.explanation.reversibility === 'irreversible').map((group) => group.category.title),
  );

  function close(): void {
    if (!cleaning) onCancel();
  }
</script>

<Dialog title={cleaning ? 'Limpando' : 'Antes de limpar'} width={640} onClose={close}>
  {#if cleaning}
    <CleaningProgress {progress} {startedAt} />
  {:else}
  <p class="lead">
    Vão ser limpos <span class="figures">{formatBytes(totalBytes)}</span> em {pluralize(itemCount, 'item', 'itens')}. Confira o que acontece
    com cada grupo.
  </p>

  {#if trashBytes > 0}
    <p class="trash-note">
      {formatBytes(trashBytes)} vão para a Lixeira: dá para recuperar, mas o espaço só volta quando você esvaziar a Lixeira.
    </p>
  {/if}

  {#if irreversibleTitles.length > 0}
    <p class="warning" role="alert">
      Sem volta: {irreversibleTitles.join(', ')}. O que for apagado nesses grupos não pode ser recuperado.
    </p>
  {/if}

  {#each groups as group (group.category.id)}
    <section class="group" style:--bed-color={colorOf(group.category.id)}>
      <header>
        <span class="swatch" aria-hidden="true"></span>
        <h3>{group.category.title}</h3>
        <span class="size figures">{formatBytes(group.totalBytes)}</span>
      </header>
      <p><strong>O que acontece.</strong> {group.category.explanation.effectOfCleaning}</p>
      <p>
        <strong>{REVERSIBILITY_LABEL[group.category.explanation.reversibility]}.</strong>
        {group.category.explanation.reversibilityDetail}
      </p>
      <p><strong>Antes de continuar.</strong> {group.category.explanation.recommendation}</p>
      <details>
        <summary class="button-disclosure">Ver {pluralize(group.items.length, 'item marcado', 'itens marcados')}</summary>
        <ul>
          {#each group.items as item (item.id)}
            <li>
              <span>{item.label}</span>
              <span class="location">{item.location}</span>
              <span class="figures">{formatBytes(item.sizeBytes)}</span>
            </li>
          {/each}
        </ul>
      </details>
    </section>
  {/each}
  {/if}

  {#snippet footer()}
    {#if !cleaning}
      <button type="button" class="button-quiet" onclick={onCancel}>Voltar</button>
    {/if}
    <button
      type="button"
      class={irreversibleTitles.length > 0 ? 'button-harm' : 'button-primary'}
      disabled={cleaning || itemCount === 0}
      onclick={onConfirm}
    >
      {cleaning ? 'Limpando…' : `Limpar ${formatBytes(totalBytes)}`}
    </button>
  {/snippet}
</Dialog>

<style>
  .lead {
    margin: 0 0 16px;
    font-size: 15px;
  }

  .trash-note {
    margin: 0 0 16px;
    color: var(--ink-soft);
  }

  .warning {
    margin: 0 0 16px;
    padding: 10px 12px;
    border-radius: var(--radius-control);
    background: color-mix(in srgb, var(--harm) 12%, transparent);
    color: var(--harm);
    font-weight: 650;
  }

  .group {
    display: grid;
    gap: 6px;
    padding: 14px 0;
    border-top: 1px solid var(--rule);
  }

  .group header {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--bed-color);
  }

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 650;
  }

  .size {
    font-size: 16px;
  }

  p {
    max-width: 66ch;
    margin: 0;
    color: var(--ink-soft);
  }

  strong {
    color: var(--ink);
  }

  ul {
    display: grid;
    gap: 4px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
  }

  .location {
    overflow: hidden;
    color: var(--ink-soft);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
