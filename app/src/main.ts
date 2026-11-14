// Entry point — mounts the root component.
// DESIGN.md §8.1 app shell (sidebar/topbar/screens) lands with P1.7.
import { mount } from 'svelte';
import App from './App.svelte';
// Token CSS layer — must load before any component styles (DESIGN.md §3–§6).
import './lib/theme/tokens.css';

const app = mount(App, {
  target: document.getElementById('app') as HTMLElement,
});

export default app;
