<script lang="ts">
  import type { Explanation } from '../api/types';
  import { REVERSIBILITY_LABEL } from '../labels';

  interface Props {
    explanation: Explanation;
  }

  let { explanation }: Props = $props();

  const answers = $derived([
    { question: 'O que é?', answer: explanation.whatItIs },
    { question: 'Por que fica tão grande?', answer: explanation.whyItGrows },
    { question: 'Se eu limpar, o que acontece?', answer: explanation.effectOfCleaning },
    { question: `Dá para desfazer? ${REVERSIBILITY_LABEL[explanation.reversibility]}.`, answer: explanation.reversibilityDetail },
    { question: 'Quanto custa recriar?', answer: explanation.recreateCost },
    { question: 'O que o Sam recomenda?', answer: explanation.recommendation },
  ]);
</script>

<dl class="answers">
  {#each answers as entry (entry.question)}
    <div>
      <dt>{entry.question}</dt>
      <dd>{entry.answer}</dd>
    </div>
  {/each}
</dl>

<style>
  .answers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 14px 28px;
    margin: 0;
    user-select: text;
    -webkit-user-select: text;
  }

  dt {
    margin-bottom: 2px;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 650;
  }

  dd {
    max-width: 62ch;
    margin: 0;
    color: var(--ink-soft);
  }
</style>
