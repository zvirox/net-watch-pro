<script lang="ts">
  // DESIGN.md §7.13 — vendored shadcn-svelte-style Switch, token-only.
  interface Props {
    /** Accessible name; rendered to the left of the control. */
    label: string;
    checked: boolean;
    /** Optional caption below the control. */
    description?: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { label, checked, description, disabled = false, onchange }: Props = $props();
</script>

<div class="switch-field">
  <div class="switch-row">
    <span class="switch-label">{label}</span>
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      {disabled}
      class="switch"
      class:switch--on={checked}
      onclick={() => {
        onchange(!checked);
      }}
    >
      <span class="switch__track"><span class="switch__thumb"></span></span>
    </button>
  </div>
  {#if description}
    <p class="switch-desc">{description}</p>
  {/if}
</div>

<style>
  .switch-field {
    display: flex;
    flex-direction: column;
    gap: var(--nwp-space-2);
  }
  .switch-row {
    display: flex;
    align-items: center;
    gap: var(--nwp-space-2);
  }
  .switch-label {
    color: var(--nwp-text);
    font-size: var(--nwp-size-label);
    font-weight: 500;
  }
  .switch-desc {
    margin: 0;
    color: var(--nwp-text-secondary);
    font-size: var(--nwp-size-caption);
  }
  .switch {
    display: inline-flex;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .switch:disabled {
    cursor: not-allowed;
  }
  .switch__track {
    display: inline-flex;
    width: 36px;
    height: 20px;
    align-items: center;
    padding: 2px;
    border: 1px solid var(--nwp-border-strong);
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface-2);
    transition: background-color var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .switch__thumb {
    width: 16px;
    height: 16px;
    border-radius: var(--nwp-radius-full);
    background: var(--nwp-surface);
    box-shadow: var(--nwp-shadow-sm);
    transition: transform var(--nwp-dur-fast) var(--nwp-ease-standard);
  }
  .switch--on .switch__track {
    border-color: var(--nwp-primary);
    background: var(--nwp-primary);
  }
  .switch--on .switch__thumb {
    transform: translateX(16px);
    background: var(--nwp-on-primary);
  }
  .switch:disabled .switch__track {
    border-color: var(--nwp-border);
    background: var(--nwp-surface-2);
  }
  .switch:disabled .switch__thumb {
    background: var(--nwp-text-disabled);
  }
  .switch:focus-visible {
    outline: 2px solid var(--nwp-primary);
    outline-offset: 2px;
    border-radius: var(--nwp-radius-full);
  }
  @media (prefers-reduced-motion: reduce) {
    .switch__track,
    .switch__thumb {
      transition: none;
    }
  }
</style>
