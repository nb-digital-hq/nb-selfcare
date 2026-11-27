<script lang="ts">
  import { numero } from '#lib/core/dia.ts';
  /** Una serie diaria, un eje, con línea de 2px y lectura al pasar el puntero o el teclado. */
  let {
    titulo,
    unidad = '',
    puntos
  }: {
    titulo: string;
    unidad?: string;
    puntos: { dia: string; valor: number | null }[];
  } = $props();
  const ancho = 320;
  const alto = 96;
  let activo = $state<number | null>(null);
  const valores = $derived(puntos.map((p) => p.valor).filter((v): v is number => v !== null));
  const min = $derived(valores.length ? Math.min(...valores) : 0);
  const max = $derived(valores.length ? Math.max(...valores) : 1);
  const x = (i: number) => (puntos.length < 2 ? ancho / 2 : (i / (puntos.length - 1)) * ancho);
  const y = (v: number) => alto - 6 - ((v - min) / (max - min || 1)) * (alto - 12);
  const trazo = $derived(
    puntos
      .map((p, i) => (p.valor === null ? null : `${x(i).toFixed(1)},${y(p.valor).toFixed(1)}`))
      .reduce<string[][]>(
        (tramos, punto) => {
          if (punto === null) tramos.push([]);
          else tramos[tramos.length - 1]?.push(punto);
          return tramos;
        },
        [[]]
      )
      .filter((t) => t.length > 0)
  );
  const ultimo = $derived([...puntos].reverse().find((p) => p.valor !== null));
  const seleccion = $derived(activo === null ? null : puntos[activo]);
  function mover(e: PointerEvent) {
    const caja = (e.currentTarget as SVGElement).getBoundingClientRect();
    const i = Math.round(((e.clientX - caja.left) / caja.width) * (puntos.length - 1));
    activo = Math.max(0, Math.min(puntos.length - 1, i));
  }
  function teclado(e: KeyboardEvent) {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const base = activo ?? puntos.length - 1;
    activo = Math.max(0, Math.min(puntos.length - 1, base + (e.key === 'ArrowRight' ? 1 : -1)));
  }
</script>

<figure class="linea">
  <figcaption>
    <span>{titulo}</span>
    <strong>{numero((seleccion ?? ultimo)?.valor ?? null, 1)} <small>{unidad}</small></strong>
    <small class="dia">{(seleccion ?? ultimo)?.dia ?? ''}</small>
  </figcaption>
  {#if valores.length > 0}
    <svg
      viewBox={`0 0 ${ancho} ${alto}`}
      preserveAspectRatio="none"
      role="slider"
      aria-label={`${titulo}: ${numero(min, 1)}–${numero(max, 1)} ${unidad}`}
      aria-valuemin={0}
      aria-valuemax={Math.max(puntos.length - 1, 0)}
      aria-valuenow={activo ?? puntos.length - 1}
      aria-valuetext={`${(seleccion ?? ultimo)?.dia ?? ''}: ${numero((seleccion ?? ultimo)?.valor ?? null, 1)} ${unidad}`}
      tabindex="0"
      onpointermove={mover}
      onpointerleave={() => (activo = null)}
      onkeydown={teclado}
      onblur={() => (activo = null)}
    >
      <line class="base" x1="0" x2={ancho} y1={alto - 1} y2={alto - 1} />
      {#each trazo as tramo}{#if tramo.length === 1}{@const [px, py] = (tramo[0] ?? '0,0').split(
            ','
          )}<circle class="aislado" cx={px} cy={py} r="3" />{:else}<polyline
            points={tramo.join(' ')}
          />{/if}{/each}
      {#if seleccion && seleccion.valor !== null && activo !== null}
        <line class="guia" x1={x(activo)} x2={x(activo)} y1="0" y2={alto} />
        <circle cx={x(activo)} cy={y(seleccion.valor)} r="4" />
      {/if}
    </svg>
  {/if}
</figure>

<style>
  .linea {
    margin: 0;
    padding: var(--sc-espacio-4);
    border-radius: var(--sc-radio-lg);
    background: var(--m3c-surface-container);
    min-width: 0;
  }
  figcaption {
    display: flex;
    align-items: baseline;
    gap: var(--sc-espacio-2);
    flex-wrap: wrap;
    color: var(--m3c-on-surface-variant);
    font-size: var(--sc-texto-sm);
  }
  figcaption strong {
    color: var(--m3c-on-surface);
    font-size: var(--sc-texto-lg);
    margin-inline-start: auto;
  }
  .dia {
    flex-basis: 100%;
    text-align: end;
  }
  svg {
    width: 100%;
    height: 6rem;
    display: block;
    margin-top: var(--sc-espacio-2);
    overflow: visible;
  }
  svg:focus-visible {
    outline: 2px solid var(--m3c-primary);
    outline-offset: 2px;
  }
  polyline {
    fill: none;
    stroke: var(--m3c-primary);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  .aislado {
    stroke: none;
  }
  .base,
  .guia {
    stroke: var(--m3c-outline-variant);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  circle {
    fill: var(--m3c-primary);
    stroke: var(--m3c-surface-container);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
</style>
