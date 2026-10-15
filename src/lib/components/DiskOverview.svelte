<script lang="ts">
  import type { DiskUsage } from '../api/types';
  import { formatBytes } from '../format';

  interface Props {
    usage: DiskUsage | null;
    foundBytes: number;
    selectedBytes: number;
    scanning: boolean;
  }

  let { usage, foundBytes, selectedBytes, scanning }: Props = $props();

  const freeingBytes = $derived(usage ? Math.min(selectedBytes, usage.usedBytes) : 0);
  const keptShare = $derived(usage && usage.totalBytes > 0 ? (usage.usedBytes - freeingBytes) / usage.totalBytes : 0);
  const freeingShare = $derived(usage && usage.totalBytes > 0 ? freeingBytes / usage.totalBytes : 0);

  const findings = $derived.by(() => {
    if (scanning) return 'Procurando o que dá para limpar…';
    if (foundBytes > 0) return `O Sam achou ${formatBytes(foundBytes)} que dá para limpar.`;
    return 'Não há nada para limpar agora.';
  });
</script>

<section class="overview" aria-label="Espaço em disco">
  <h1 class="headline">
    {#if usage}
      Seu Mac tem {formatBytes(usage.availableBytes)} livres de {formatBytes(usage.totalBytes)}. {findings}
    {:else}
      Medindo o disco…
    {/if}
  </h1>

  <div
    class="bar"
    role="meter"
    aria-label="Espaço ocupado"
    aria-valuemin={0}
    aria-valuemax={usage?.totalBytes ?? 0}
    aria-valuenow={usage?.usedBytes ?? 0}
  >
    <span class="kept" style:flex-basis="{keptShare * 100}%"></span>
    <span class="freeing" style:flex-basis="{freeingShare * 100}%"></span>
  </div>

  {#if usage}
    <ul class="legend">
      <li><span class="swatch kept"></span>Ocupado</li>
      <li>
        <span class="swatch freeing"></span>Liberado ao limpar o que está marcado
        <span class="figures">{formatBytes(freeingBytes)}</span>
      </li>
      <li><span class="swatch free"></span>Livre</li>
    </ul>
  {/if}
</section>

<style>
  .overview {
    display: grid;
    gap: 14px;
  }

  .headline {
    max-width: 34em;
    margin: 0;
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 500;
    line-height: 1.28;
    letter-spacing: -0.005em;
    font-variant-numeric: lining-nums;
    text-wrap: pretty;
  }

  .bar {
    display: flex;
    height: 12px;
    border-radius: 6px;
    background: var(--paper);
    box-shadow: inset 0 0 0 1px var(--rule);
    overflow: hidden;
  }

  .bar > span {
    flex-grow: 0;
    flex-shrink: 0;
    transition: flex-basis var(--duration-settle) var(--ease);
  }

  .kept {
    background: var(--soil);
  }

  .freeing {
    background: var(--sprout);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 20px;
    margin: 0;
    padding: 0;
    color: var(--ink-soft);
    font-size: 12.5px;
    list-style: none;
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .legend .figures {
    color: var(--ink);
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .swatch.free {
    background: var(--paper);
    box-shadow: inset 0 0 0 1px var(--rule);
  }
</style>
