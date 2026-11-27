<script lang="ts">
  import { numero } from '#lib/core/dia.ts';
  let {
    valor,
    maximo = 100,
    etiqueta,
    unidad = '',
    texto,
    animar = true
  }: {
    valor: number;
    maximo?: number;
    etiqueta: string;
    unidad?: string;
    texto?: string;
    animar?: boolean;
  } = $props();
  const proporcion = $derived(Math.min(1, Math.max(0, valor / Math.max(1, maximo))));
</script>

<div
  class="anillo"
  class:sinAnimacion={!animar}
  role="progressbar"
  aria-label={etiqueta}
  aria-valuemin="0"
  aria-valuemax={Math.max(1, maximo)}
  aria-valuenow={Math.min(Math.max(1, maximo), Math.max(0, valor))}
  aria-valuetext={`${texto ?? numero(valor)} ${unidad}`}
>
  <svg viewBox="0 0 120 120" aria-hidden="true"
    ><circle class="pista" cx="60" cy="60" r="51" /><circle
      class="avance"
      cx="60"
      cy="60"
      r="51"
      pathLength="100"
      stroke-dasharray="100"
      stroke-dashoffset={100 - proporcion * 100}
    /></svg
  >
  <div>
    <strong>{texto ?? numero(valor)}</strong>{#if unidad}<span>{unidad}</span>{/if}
  </div>
</div>

<style>
  .anillo {
    position: relative;
    width: var(--anillo-tamano, 9rem);
    aspect-ratio: 1;
    flex-shrink: 0;
    color: var(--m3c-primary);
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 9;
  }
  .pista {
    stroke: var(--m3c-surface-container-highest);
  }
  .avance {
    stroke: currentColor;
    stroke-linecap: round;
    transition: stroke-dashoffset var(--sc-duracion-media) var(--sc-curva);
  }
  .anillo > div {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--m3c-on-surface);
  }
  strong {
    font-size: var(--anillo-texto, var(--sc-texto-2xl));
    font-weight: 650;
    letter-spacing: -0.04em;
    font-variant-numeric: tabular-nums;
  }
  span {
    font-size: var(--sc-texto-xs);
    color: var(--m3c-on-surface-variant);
  }
  .sinAnimacion .avance {
    transition: none;
  }
</style>
