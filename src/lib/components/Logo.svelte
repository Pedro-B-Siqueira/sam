<script lang="ts">
  interface Props {
    size?: number;
  }

  let { size = 32 }: Props = $props();
</script>

<!-- Same drawing as app-icon.svg, so the window and the Dock show one mark. -->
<svg viewBox="64 64 896 896" width={size} height={size} aria-hidden="true">
  <rect x="64" y="64" width="896" height="896" rx="200" fill="#1b1a18" />
  <path d="M512 800 V470" stroke="#72c68c" stroke-width="56" stroke-linecap="round" fill="none" />
  <path d="M512 590 C 372 590 268 492 258 340 C 410 340 512 430 512 590 Z" fill="#72c68c" />
  <path d="M512 500 C 540 340 656 246 800 256 C 800 414 684 506 512 500 Z" fill="#a6e0b5" />
  <path d="M340 800 H684" stroke="#d97757" stroke-width="48" stroke-linecap="round" />
</svg>
