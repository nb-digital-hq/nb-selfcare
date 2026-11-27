<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { textosExtra } from '#lib/i18n/sincronizacion.ts';
  const extra = $derived(textosExtra());
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  import EstadoCarga from '../EstadoCarga.svelte';
  import DialogoConfirmar from '../DialogoConfirmar.svelte';
  import Linea from './Linea.svelte';

  let { vista }: { vista: 'semana' | 'tendencias' | 'historial' } = $props();
  const e = $derived(extra.evolucion);
  const acciones = new Acciones();
  const datos = new Recurso<ipc.Evolucion>();
  const movimientos = new Recurso<ipc.Movimiento[]>();
  let rango = $state(30);
  const hoy = diaLocal();
  const haceDias = (n: number) => {
    const d = new Date(`${hoy}T12:00:00`);
    d.setDate(d.getDate() - n);
    return d.toISOString().slice(0, 10);
  };
  let desde = $state(haceDias(13));
  let hasta = $state(hoy);
  let tipo = $state<ipc.TipoDeMovimiento | 'todos'>('todos');
  let borrando = $state<ipc.Movimiento | null>(null);

  const UNIDADES: Record<ipc.Medida, string> = {
    kcal: 'kcal',
    proteinaG: 'g',
    aguaMl: 'ml',
    pesoKg: 'kg',
    suenoH: 'h',
    pasos: '',
    animo: '/5',
    energia: '/5',
    estres: '/5',
    entrenoMin: 'min',
    habitos: '',
    cafeinaMg: 'mg'
  };
  /** Sentido deseable de cada medida; `null` cuando subir o bajar no es en sí mejor. */
  const MEJOR_SI_SUBE: Partial<Record<ipc.Medida, boolean>> = {
    aguaMl: true,
    suenoH: true,
    pasos: true,
    animo: true,
    energia: true,
    estres: false,
    entrenoMin: true,
    habitos: true
  };
  const nombre = (m: ipc.Medida) => e[m];
  const TIPOS: ipc.TipoDeMovimiento[] = [
    'comida',
    'agua',
    'peso',
    'entreno',
    'diario',
    'cafeina',
    'sintoma',
    'signo',
    'digestion',
    'sesion'
  ];

  const cargar = () => {
    if (vista === 'historial') void movimientos.cargar(() => ipc.historial(desde, hasta));
    else void datos.cargar(() => ipc.evolucion(vista === 'semana' ? 14 : rango));
  };
  $effect(() => {
    void cambios.version;
    void vista;
    void rango;
    cargar();
  });
  onDestroy(() => {
    datos.cancelar();
    movimientos.cancelar();
  });

  const SERIES: ipc.Medida[] = [
    'pesoKg',
    'kcal',
    'proteinaG',
    'aguaMl',
    'suenoH',
    'pasos',
    'animo',
    'energia',
    'estres',
    'entrenoMin',
    'habitos',
    'cafeinaMg'
  ];
  const clave: Record<ipc.Medida, keyof ipc.DiaEvolucion> = {
    kcal: 'kcal',
    proteinaG: 'proteinaG',
    aguaMl: 'aguaMl',
    pesoKg: 'pesoKg',
    suenoH: 'suenoH',
    pasos: 'pasos',
    animo: 'animo',
    energia: 'energia',
    estres: 'estres',
    entrenoMin: 'entrenoMin',
    habitos: 'habitos',
    cafeinaMg: 'cafeinaMg'
  };
  const serie = (dias: ipc.DiaEvolucion[], m: ipc.Medida) =>
    dias.map((d) => ({ dia: d.dia, valor: d[clave[m]] as number | null }));

  function frase(r: ipc.Relacion) {
    const base = r.coeficiente > 0 ? e.juntos : e.opuestos;
    return (
      base
        .replace('{a}', nombre(r.causaPosible).toLowerCase())
        .replace('{b}', nombre(r.efecto).toLowerCase()) +
      (r.diaAnterior ? ` ${e.diaSiguiente}` : '')
    );
  }
  const filtrados = $derived(
    (movimientos.datos ?? []).filter((m) => tipo === 'todos' || m.tipo === tipo)
  );
  const hora = (iso: string) => iso.slice(11, 16);
  /** Pasos, energía, agua, hábitos y cafeína se leen en enteros; el resto con un decimal. */
  const dec = (m: ipc.Medida) =>
    ['pasos', 'kcal', 'aguaMl', 'habitos', 'cafeinaMg'].includes(m) ? 0 : 1;
</script>

{#if vista === 'semana'}
  <section class="panel">
    <h2>{e.semana}</h2>
    <p class="ayuda">{e.semanaSub}</p>
    <EstadoCarga
      cargando={datos.cargando && !datos.datos}
      fallo={datos.fallo}
      reintentar={cargar}
    />
    {#if datos.datos}
      {@const d = datos.datos}
      <p class="activos">{e.diasActivos.replace('{n}', String(d.diasActivos))}</p>
      {#if d.semana.length === 0}<p class="ayuda">{e.sinDatosSemana}</p>{/if}
      <div class="comparaciones">
        {#each d.semana as c (c.medida)}
          {@const delta = c.esta !== null && c.anterior !== null ? c.esta - c.anterior : null}
          {@const sentido = MEJOR_SI_SUBE[c.medida]}
          <div class="comparacion">
            <span>{nombre(c.medida)}</span>
            <strong>{numero(c.esta, dec(c.medida))} <small>{UNIDADES[c.medida]}</small></strong>
            <small
              >{e.anterior}: {c.anterior === null
                ? '—'
                : `${numero(c.anterior, dec(c.medida))} ${UNIDADES[c.medida]}`}</small
            >
            {#if delta !== null && Math.abs(delta) > 0.05}
              <small
                class="delta"
                class:bien={sentido !== undefined && delta > 0 === sentido}
                class:revisar={sentido !== undefined && delta > 0 !== sentido}
                >{delta > 0 ? '▲' : '▼'}
                {numero(Math.abs(delta), dec(c.medida))}
                {UNIDADES[c.medida]}{#if sentido !== undefined}
                  · {delta > 0 === sentido ? e.mejor : e.peor}{/if}</small
              >
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>
  {#if datos.datos}
    <section class="panel">
      <h2>{e.relaciones}</h2>
      <p class="ayuda">{e.relacionesAyuda}</p>
      {#if datos.datos.relaciones.length === 0}<p class="ayuda">{e.sinRelaciones}</p>{/if}
      <ul class="relaciones">
        {#each datos.datos.relaciones as r}
          <li>
            <span>{frase(r)}</span><small
              >r = {numero(r.coeficiente, 2)} · {e.muestra.replace('{n}', String(r.muestra))}</small
            >
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{:else if vista === 'tendencias'}
  <section class="panel">
    <div class="cabecera">
      <div>
        <h2>{e.tendencias}</h2>
        <p class="ayuda">{e.tendenciasSub}</p>
      </div>
      <div class="rangos" role="group" aria-label={e.rango}>
        {#each [30, 90, 180] as n}<button
            type="button"
            aria-pressed={rango === n}
            class:seleccionado={rango === n}
            onclick={() => (rango = n)}
            >{n === 30 ? e.dias30 : n === 90 ? e.dias90 : e.dias180}</button
          >{/each}
      </div>
    </div>
    <EstadoCarga
      cargando={datos.cargando && !datos.datos}
      fallo={datos.fallo}
      reintentar={cargar}
    />
    {#if datos.datos}
      <div class="lineas">
        {#each SERIES.filter((m) => datos.datos?.dias.some((d) => d[clave[m]] !== null)) as m (m)}
          <Linea titulo={nombre(m)} unidad={UNIDADES[m]} puntos={serie(datos.datos.dias, m)} />
        {/each}
      </div>
    {/if}
  </section>
{:else}
  <section class="panel">
    <h2>{e.historial}</h2>
    <p class="ayuda">{e.historialSub}</p>
    <form
      class="filtros"
      onsubmit={(ev) => {
        ev.preventDefault();
        cargar();
      }}
    >
      <label>{e.desde}<input type="date" bind:value={desde} max={hasta} required /></label>
      <label>{e.hasta}<input type="date" bind:value={hasta} min={desde} max={hoy} required /></label
      >
      <MenuSelector
        rotulo={e.tipo}
        bind:valor={tipo}
        opciones={[
          { valor: 'todos' as typeof tipo, etiqueta: e.todos },
          ...TIPOS.map((t) => ({ valor: t as typeof tipo, etiqueta: e[t] }))
        ]}
      />
      <button>{textos.comun.ver}</button>
    </form>
    <EstadoCarga
      cargando={movimientos.cargando && !movimientos.datos}
      fallo={movimientos.fallo}
      reintentar={cargar}
    />
    {#if movimientos.datos && filtrados.length === 0}<p class="ayuda">{e.sinMovimientos}</p>{/if}
    {#if filtrados.length > 0}
      <div class="tabla-contenedor">
        <table class="tabla">
          <thead
            ><tr
              ><th scope="col">{e.fecha}</th><th scope="col">{e.tipo}</th><th scope="col"></th><th
                scope="col"
              ></th><th scope="col"><span class="sr-only">{e.borrar}</span></th></tr
            ></thead
          >
          <tbody>
            {#each filtrados as m (m.tipo + m.id)}
              <tr>
                <td>{m.dia} {hora(m.instante)}</td>
                <td>{e[m.tipo]}</td>
                <td>{m.titulo}</td>
                <td>{numero(m.valor, 1)}{m.detalle ? ` · ${m.detalle}` : ''}</td>
                <td
                  ><button
                    type="button"
                    class="discreto"
                    aria-label={`${e.borrar}: ${e[m.tipo]} ${m.dia}`}
                    disabled={acciones.ocupado}
                    onclick={() => (borrando = m)}>{e.borrar}</button
                  ></td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
  <DialogoConfirmar
    abierto={borrando !== null}
    titulo={e.borrar}
    mensaje={e.borrarConfirmar}
    textoConfirmar={e.borrar}
    destructivo
    ocupado={acciones.ocupado}
    onconfirmar={() => {
      const m = borrando;
      borrando = null;
      if (m) void acciones.hacer(() => ipc.historialBorrar(m.tipo, m.id));
    }}
    oncerrar={() => (borrando = null)}
  />
{/if}

<style>
  .activos {
    color: var(--m3c-on-surface-variant);
  }
  .comparaciones,
  .lineas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
    gap: var(--sc-espacio-3);
    margin-top: var(--sc-espacio-4);
  }
  .comparacion {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-1);
    padding: var(--sc-espacio-4);
    border-radius: var(--sc-radio-lg);
    background: var(--m3c-surface-container);
  }
  .comparacion > span,
  .comparacion > small {
    color: var(--m3c-on-surface-variant);
    font-size: var(--sc-texto-sm);
  }
  .comparacion strong {
    font-size: var(--sc-texto-xl);
  }
  .comparacion strong small {
    font-size: var(--sc-texto-sm);
    font-weight: 500;
    color: var(--m3c-on-surface-variant);
  }
  .delta.bien {
    color: var(--m3c-primary);
  }
  .delta.revisar {
    color: var(--m3c-tertiary);
  }
  .relaciones {
    list-style: none;
    padding: 0;
    margin: var(--sc-espacio-3) 0 0;
    display: grid;
    gap: var(--sc-espacio-2);
  }
  .relaciones li {
    display: flex;
    justify-content: space-between;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-3) var(--sc-espacio-4);
    border-radius: var(--sc-radio-md);
    background: var(--m3c-surface-container);
  }
  .relaciones small {
    color: var(--m3c-on-surface-variant);
    white-space: nowrap;
  }
  .cabecera {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--sc-espacio-4);
    flex-wrap: wrap;
  }
  .rangos {
    display: flex;
    gap: var(--sc-espacio-1);
  }
  .rangos button.seleccionado {
    background: var(--m3c-secondary-container);
    color: var(--m3c-on-secondary-container);
  }
  .filtros {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sc-espacio-3);
    align-items: flex-end;
    margin: var(--sc-espacio-4) 0;
  }
  .filtros label {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-1);
    font-size: var(--sc-texto-sm);
  }
</style>
