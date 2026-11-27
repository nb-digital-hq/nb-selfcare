<script lang="ts">
  import { numero } from '#lib/core/dia.ts';
  let {
    titulo,
    valor,
    meta,
    unidad = ''
  }: { titulo: string; valor: number | null; meta?: number; unidad?: string } = $props();
</script>

<div class="metrica">
  <span>{titulo}</span>
  <div><strong>{numero(valor, 1)}</strong><small>{unidad}</small></div>
  {#if meta !== undefined}<progress max={Math.max(meta, 1)} value={valor ?? 0} aria-label={titulo}
    ></progress><small>{numero(meta)} {unidad}</small>{/if}
</div>

<style>
  .metrica {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-2);
    padding: var(--sc-espacio-5);
    border-radius: var(--sc-radio-lg);
    background: var(--m3c-surface-container);
    border: 1px solid color-mix(in srgb, var(--m3c-outline-variant) 55%, transparent);
    min-width: 0;
  }
  .metrica > span {
    color: var(--m3c-on-surface-variant);
    font-size: var(--sc-texto-sm);
  }
  .metrica strong {
    font-size: var(--sc-texto-2xl);
    font-weight: 650;
    letter-spacing: -0.04em;
  }
  .metrica small {
    color: var(--m3c-on-surface-variant);
    margin-left: var(--sc-espacio-2);
  }
  progress {
    width: 100%;
    height: 0.35rem;
    accent-color: var(--m3c-primary);
  }
</style>
