<script lang="ts">
  import { openTrash } from '../api/commands';
  import { describeFailure, reportError, type FailureCause } from '../api/logger';
  import type { CleanOutcome, DiskUsage } from '../api/types';
  import { formatBytes, pluralize } from '../format';
  import Dialog from './Dialog.svelte';

  interface Props {
    outcomes: CleanOutcome[];
    diskBefore: DiskUsage | null;
    diskAfter: DiskUsage | null;
    /** What went to the Trash: it only frees space once the Trash is emptied. */
    trashedBytes: number;
    /** "Relatório salvo em…" or why saving failed, after the user asked to save. */
    reportStatus: string | null;
    onSaveReport: () => void;
    onClose: () => void;
  }

  let { outcomes, diskBefore, diskAfter, trashedBytes, reportStatus, onSaveReport, onClose }: Props = $props();

  let trashError = $state<string | null>(null);

  async function showTrash(): Promise<void> {
    trashError = null;
    await openTrash().catch((cause: FailureCause) => {
      reportError('abrir a Lixeira', cause);
      trashError = `Não deu para abrir a Lixeira: ${describeFailure(cause)}`;
    });
  }

  const done = $derived(outcomes.filter((outcome) => outcome.status === 'done'));
  const failed = $derived(outcomes.flatMap((outcome) => (outcome.status === 'failed' ? [outcome] : [])));
  const commandOutputs = $derived(outcomes.flatMap((outcome) => (outcome.status === 'done' && outcome.output ? [outcome] : [])));
  const estimatedBytes = $derived(done.reduce((total, outcome) => total + outcome.sizeBytes, 0));
  const measuredBytes = $derived(diskBefore && diskAfter ? diskAfter.availableBytes - diskBefore.availableBytes : null);
</script>

<Dialog title="Limpeza concluída" onClose={onClose}>
  <p class="headline">
    {#if measuredBytes !== null && measuredBytes > 0}
      Seu Mac ganhou {formatBytes(measuredBytes)} de espaço livre.
    {:else}
      O Sam limpou cerca de {formatBytes(estimatedBytes)}.
    {/if}
  </p>

  {#if diskBefore && diskAfter}
    <p class="soft">
      Livre antes: {formatBytes(diskBefore.availableBytes)}. Livre agora: {formatBytes(diskAfter.availableBytes)}. A medição pode diferir
      da estimativa porque o macOS devolve parte do espaço aos poucos e outros apps continuam gravando no disco.
    </p>
  {/if}

  <p>
    {pluralize(done.length, 'item limpo', 'itens limpos')}{failed.length > 0 ? `; ${pluralize(failed.length, 'falhou', 'falharam')}` : ''}.
  </p>

  {#if trashedBytes > 0}
    <div class="trash" role="note">
      <p>
        {formatBytes(trashedBytes)} foram para a Lixeira e ainda ocupam o disco. Confira se não precisa de nada lá e esvazie a Lixeira para
        liberar esse espaço.
      </p>
      <button type="button" class="button-quiet" onclick={showTrash}>Abrir a Lixeira</button>
      {#if trashError}<p class="trash-error">{trashError}</p>{/if}
    </div>
  {/if}

  {#if failed.length > 0}
    <ul class="failures">
      {#each failed as outcome (outcome.itemId)}
        <li><strong>{outcome.label}</strong>: {outcome.message}</li>
      {/each}
    </ul>
  {/if}

  {#each commandOutputs as outcome (outcome.itemId)}
    <details>
      <summary class="button-disclosure">O que {outcome.label} respondeu</summary>
      <pre>{outcome.output}</pre>
    </details>
  {/each}

  {#if reportStatus}<p class="soft" role="status">{reportStatus}</p>{/if}

  {#snippet footer()}
    <button type="button" class="button-quiet" onclick={onSaveReport}>Salvar relatório</button>
    <button type="button" class="button-primary" onclick={onClose}>Fechar</button>
  {/snippet}
</Dialog>

<style>
  .headline {
    margin: 0 0 10px;
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 550;
    line-height: 1.3;
  }

  p {
    max-width: 66ch;
  }

  .soft {
    color: var(--ink-soft);
  }

  .trash {
    display: grid;
    justify-items: start;
    gap: 8px;
    margin: 12px 0;
    padding: 12px 14px;
    border-radius: var(--radius-control);
    background: var(--paper);
  }

  .trash p {
    margin: 0;
  }

  .trash-error {
    color: var(--caution);
  }

  .failures {
    padding-left: 18px;
    color: var(--harm);
  }

  pre {
    max-height: 200px;
    padding: 10px 12px;
    border-radius: var(--radius-control);
    background: var(--paper-sunk);
    font-size: 12px;
    overflow: auto;
    white-space: pre-wrap;
  }
</style>
