<script lang="ts">
  import type { CategoryInfo, CategoryScan } from '../api/types';
  import { formatBytes } from '../format';
  import { labelFor, plantsFor, type GardenPlant } from '../garden';
  import { plotColor } from '../labels';
  import { columnsPerRow, layoutRows, plantsPerColumn, rowsHeight, type Bed } from '../plot';
  import type { CheckState, Selection } from '../selection';

  interface Props {
    categories: CategoryInfo[];
    scans: ReadonlyMap<string, CategoryScan>;
    selection: Selection;
    scanning: boolean;
    disabled: boolean;
    onSetItems: (categoryId: string, itemIds: string[], selected: boolean) => void;
  }

  let { categories, scans, selection, scanning, disabled, onSetItems }: Props = $props();

  /** One row of beds gets this height; with more rows each one is shorter, so the plot stays compact. */
  const SINGLE_ROW_HEIGHT = 232;
  const WRAPPED_ROW_HEIGHT = 176;
  const SPACING = { columnGap: 6, plantGap: 3, rowGap: 6, signHeight: 20, minColumnWidth: 76, minPlantHeight: 46, maxColumnsPerRow: 7 };
  /** Below this height the name shares a line with the selection marker and must stop before it. */
  const ROOMY_HEIGHT = 64;

  let width = $state(0);

  const filledCount = $derived(categories.filter((category) => (scans.get(category.id)?.items.length ?? 0) > 0).length);
  const wraps = $derived(width > 0 && filledCount > columnsPerRow(width, SPACING));
  const rowHeight = $derived(wraps ? WRAPPED_ROW_HEIGHT : SINGLE_ROW_HEIGHT);
  const plotHeight = $derived(rowsHeight(filledCount, width, rowHeight, SPACING));

  const plantsByCategory = $derived(
    new Map(
      categories.map((category) => [
        category.id,
        plantsFor(category.id, scans.get(category.id)?.items ?? [], plantsPerColumn(rowHeight, SPACING)),
      ]),
    ),
  );

  const beds = $derived<Bed[]>(
    categories.map((category) => ({
      key: category.id,
      plants: (plantsByCategory.get(category.id) ?? []).map((plant) => ({ key: plant.key, weight: plant.sizeBytes })),
    })),
  );

  function checkState(categoryId: string, plant: GardenPlant): CheckState {
    const selectedIds = selection.get(categoryId);
    const count = plant.itemIds.filter((itemId) => selectedIds?.has(itemId)).length;
    if (count === 0) return 'none';
    return count === plant.itemIds.length ? 'all' : 'some';
  }

  const cells = $derived(
    layoutRows(beds, width, rowHeight, SPACING).flatMap((placed) => {
      const categoryIndex = categories.findIndex((category) => category.id === placed.bedKey);
      const plant = plantsByCategory.get(placed.bedKey)?.find((candidate) => candidate.key === placed.plantKey);
      if (!plant || categoryIndex < 0) return [];
      return [
        {
          ...placed,
          plant,
          color: plotColor(categoryIndex),
          state: checkState(placed.bedKey, plant),
          label: labelFor(placed.width, placed.height),
          cramped: placed.height < ROOMY_HEIGHT,
        },
      ];
    }),
  );

  /** One sign per column, naming its category, so repeated colors never leave a column unidentified. */
  const signs = $derived.by(() => {
    const byCategory = new Map<string, { key: string; x: number; y: number; width: number; title: string; color: string }>();
    for (const cell of cells) {
      const known = byCategory.get(cell.bedKey);
      if (known && known.y <= cell.y - SPACING.signHeight) continue;
      const title = categories.find((category) => category.id === cell.bedKey)?.shortTitle ?? '';
      byCategory.set(cell.bedKey, { key: cell.bedKey, x: cell.x, y: cell.y - SPACING.signHeight, width: cell.width, title, color: cell.color });
    }
    return [...byCategory.values()];
  });
</script>

<figure class="plot-figure">
  <div class="plot" style:height="{plotHeight}px" bind:clientWidth={width}>
    {#each signs as sign (sign.key)}
      <span
        class="sign"
        style:left="{sign.x}px"
        style:top="{sign.y}px"
        style:width="{sign.width}px"
        style:height="{SPACING.signHeight}px"
        style:--bed-color={sign.color}
        title={categories.find((category) => category.id === sign.key)?.title}
      >
        <span class="sign-swatch" aria-hidden="true"></span><span class="sign-text">{sign.title}</span>
      </span>
    {/each}
    {#each cells as cell (`${cell.bedKey}/${cell.plantKey}`)}
      <button
        type="button"
        class="cell"
        class:selected={cell.state === 'all'}
        class:partial={cell.state === 'some'}
        class:cramped={cell.cramped}
        style:left="{cell.x}px"
        style:top="{cell.y}px"
        style:width="{cell.width}px"
        style:height="{cell.height}px"
        style:--bed-color={cell.color}
        aria-pressed={cell.state === 'all' ? true : cell.state === 'some' ? 'mixed' : false}
        aria-label="{cell.plant.label}, {formatBytes(cell.plant.sizeBytes)}"
        title="{cell.plant.label}: {formatBytes(cell.plant.sizeBytes)}"
        {disabled}
        onclick={() => onSetItems(cell.bedKey, cell.plant.itemIds, cell.state !== 'all')}
      >
        {#if cell.label.showsMark}
          <span class="mark" aria-hidden="true"></span>
        {/if}
        {#if cell.label.showsName || cell.label.showsSize}
          <span class="label">
            {#if cell.label.showsName}
              <span class="cell-name">{cell.plant.label}</span>
            {/if}
            {#if cell.label.showsSize}
              <span class="cell-size figures">{formatBytes(cell.plant.sizeBytes)}</span>
            {/if}
          </span>
        {/if}
      </button>
    {:else}
      <p class="empty">{scanning ? 'Medindo o que dá para limpar…' : 'Nada para limpar agora.'}</p>
    {/each}
  </div>
  {#if cells.length > 0}
    <figcaption>
      Cada coluna é uma categoria, mais larga quanto mais espaço ocupa; as bem pequenas ganham um tamanho mínimo para dar para ler e
      clicar. As áreas cheias vão ser limpas, as listradas ficam como estão.
    </figcaption>
  {/if}
</figure>

<style>
  .plot-figure {
    margin: 0;
  }

  .plot {
    position: relative;
    border-radius: var(--radius-panel);
    background: var(--paper-sunk);
    overflow: hidden;
  }

  .sign {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 4px 3px;
    overflow: hidden;
    color: var(--ink-soft);
    font-size: 11.5px;
    font-weight: 650;
  }

  .sign-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sign-swatch {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 2px;
    background: var(--bed-color);
  }

  .cell {
    position: absolute;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 1px;
    padding: 7px 9px;
    border: 0;
    border-radius: var(--radius-small);
    background: repeating-linear-gradient(
      135deg,
      color-mix(in srgb, var(--bed-color) 24%, transparent) 0 5px,
      color-mix(in srgb, var(--bed-color) 9%, transparent) 5px 10px
    );
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--bed-color) 55%, transparent);
    color: var(--ink);
    text-align: left;
    overflow: hidden;
    cursor: pointer;
    transition:
      left var(--duration-settle) var(--ease),
      top var(--duration-settle) var(--ease),
      width var(--duration-settle) var(--ease),
      height var(--duration-settle) var(--ease),
      background-color var(--duration-quick) var(--ease);
  }

  .cell.selected {
    background: var(--bed-color);
    box-shadow: none;
    color: var(--plot-ink);
  }

  .cell:hover:not(:disabled) {
    filter: brightness(1.07);
  }

  .cell:disabled {
    cursor: default;
  }

  /* A checkbox-like marker so the plot reads as something to tick, not just a chart. */
  .mark {
    position: absolute;
    top: 7px;
    right: 7px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--paper) 80%, transparent);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--bed-color) 80%, var(--ink));
  }

  .cell.selected .mark {
    background: var(--plot-ink);
    box-shadow: none;
  }

  .cell.partial .mark::after {
    content: '';
    position: absolute;
    top: 7px;
    left: 4px;
    width: 8px;
    height: 2px;
    background: color-mix(in srgb, var(--bed-color) 80%, var(--ink));
  }

  .cell.selected .mark::after {
    content: '';
    position: absolute;
    top: 3.5px;
    left: 5.5px;
    width: 4px;
    height: 7px;
    border-right: 2px solid var(--bed-color);
    border-bottom: 2px solid var(--bed-color);
    transform: rotate(45deg);
  }

  .cell-name {
    max-inline-size: 100%;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: 12.5px;
    font-weight: 650;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    max-inline-size: 100%;
  }

  .cell.cramped .cell-name {
    padding-right: 20px;
  }

  .cell-size {
    font-size: 12px;
    opacity: 0.85;
    white-space: nowrap;
  }

  .empty {
    display: grid;
    place-items: center;
    height: 100%;
    margin: 0;
    color: var(--ink-soft);
  }

  figcaption {
    margin-top: 8px;
    color: var(--ink-soft);
    font-size: 12.5px;
  }
</style>
