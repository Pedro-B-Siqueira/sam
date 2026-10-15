import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach, vi } from 'vitest';

// There is no Tauri runtime in jsdom; failures are still logged by the code under test.
vi.mock('@tauri-apps/plugin-log', () => ({
  error: vi.fn(async () => undefined),
}));

afterEach(() => {
  cleanup();
});
