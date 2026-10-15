<script lang="ts">
  import { ICON_PATHS, type IconName } from '../icons';

  interface Props {
    name: IconName;
    size?: number;
  }

  let { name, size = 15 }: Props = $props();
</script>

<svg
  viewBox="0 0 16 16"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={ICON_PATHS[name]} />
</svg>
