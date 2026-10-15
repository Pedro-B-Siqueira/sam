<script lang="ts">
  import { onMount } from 'svelte';
  import type { CleanProgress } from '../api/types';
  import { formatBytes, formatElapsed, pluralize } from '../format';

  interface Props {
    /** `null` until the backend sends its first update. */
    progress: CleanProgress | null;
    startedAt: number;
  }

  let { progress, startedAt }: Props = $props();

  const TICK_MILLISECONDS = 1000;
  let now = $state(Date.now());

  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), TICK_MILLISECONDS);
    return () => clearInterval(timer);
  });

  // Weighted by size: one 16 GB folder is most of the work, not one item in fourteen.
  const percent = $derived.by(() => {
    if (!progress) return 0;
    if (progress.totalBytes > 0) return Math.round((progress.completedBytes / progress.totalBytes) * 100);
    return progress.totalItems > 0 ? Math.round((progress.completedItems / progress.totalItems) * 100) : 0;
  });
  const remainingBytes = $derived(progress ? progress.totalBytes - progress.completedBytes : 0);
  const remainingItems = $derived(progress ? progress.totalItems - progress.completedItems : 0);
</script>

<div class="cleaning">
  <p class="headline"><span class="figures">{percent}%</span> limpo</p>
  <div class="bar" role="progressbar" aria-label="Andamento da limpeza" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
    <span class="fill" style:width="{percent}%"></span>
  </div>

  {#if progress}
    <p>
      {progress.completedItems} de {pluralize(progress.totalItems, 'item', 'itens')} prontos. Faltam {formatBytes(remainingBytes)} em
      {pluralize(remainingItems, 'item', 'itens')}.
    </p>
    {#if progress.currentLabel}
      <p class="current">Limpando agora: <strong>{progress.currentLabel}</strong></p>
    {/if}
  {:else}
    <p>Preparando a limpeza…</p>
  {/if}

  <p class="soft">
    Tempo decorrido: <span class="figures">{formatElapsed(now - startedAt)}</span>. Itens grandes, como builds e o Docker, podem levar
    alguns minutos cada; mantenha a janela aberta até terminar.
  </p>
</div>

<style>
  .cleaning {
    display: grid;
    gap: 10px;
  }

  p {
    max-width: 62ch;
    margin: 0;
  }

  .headline {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 600;
  }

  .bar {
    height: 12px;
    border-radius: 6px;
    background: var(--paper-sunk);
    overflow: hidden;
  }

  /* The moving stripes show work is happening even while one long item holds the percentage still. */
  .fill {
    display: block;
    height: 100%;
    min-width: 12px;
    border-radius: 6px;
    background: repeating-linear-gradient(
      135deg,
      var(--moss) 0 10px,
      color-mix(in srgb, var(--moss) 78%, var(--paper)) 10px 20px
    );
    background-size: 28px 28px;
    transition: width var(--duration-settle) var(--ease);
    animation: stripes 900ms linear infinite;
  }

  @keyframes stripes {
    to {
      background-position: 28px 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      animation: none;
    }
  }

  .current {
    overflow-wrap: anywhere;
  }

  .soft {
    color: var(--ink-soft);
    font-size: 12.5px;
  }
</style>
