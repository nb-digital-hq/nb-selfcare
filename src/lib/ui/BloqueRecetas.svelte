<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { textosExtra } from '#lib/i18n/sincronizacion.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  import EstadoCarga from './EstadoCarga.svelte';
  import BotonEliminar from './BotonEliminar.svelte';
  /** Recetas: registrar raciones hoy o guardar como receta la comida elegida arriba. */
  let { dia, comida }: { dia: string; comida: string } = $props();
  const t = $derived(textosExtra().recetas);
  const acciones = new Acciones();
  const lista = new Recurso<ipc.Receta[]>();
  let nombre = $state('');
  let raciones = $state(4);
  let porciones = $state<Record<number, number>>({});
  $effect(() => {
    void cambios.version;
    void lista.cargar(ipc.recetas);
  });
  onDestroy(() => lista.cancelar());
</script>

<section class="panel">
  <h2>{textos.ui.recetas}</h2>
  <p class="ayuda">{t.sub}</p>
  <form
    class="campos"
    onsubmit={(e) => {
      e.preventDefault();
      void acciones.hacer(async () => {
        await ipc.recetaCrear(nombre, raciones, dia, comida);
        nombre = '';
        await lista.cargar(ipc.recetas);
      });
    }}
  >
    <label>{t.nombre}<input bind:value={nombre} required maxlength="80" /></label>
    <label
      >{t.raciones}<input type="number" bind:value={raciones} min="1" max="50" required /></label
    >
    <button disabled={acciones.ocupado}>{t.crear}</button>
  </form>
</section>
<section class="panel">
  <EstadoCarga
    cargando={lista.cargando && !lista.datos}
    fallo={lista.fallo}
    reintentar={() => void lista.cargar(ipc.recetas)}
  />
  {#if lista.datos?.length === 0}<p class="ayuda">{t.vacio}</p>{/if}
  {#if lista.datos?.length}<p class="ayuda">{t.soloHoy}</p>{/if}
  {#each lista.datos ?? [] as r (r.id)}
    <article class="receta">
      <div class="datos">
        <strong>{r.nombre}</strong>
        <p class="ayuda">
          {numero(r.kcal)}
          {textos.unidades.kcal} · {numero(r.proteina, 1)}
          {textos.unidades.g}
          {textos.ui.proteina.toLowerCase()} · {t.porRacion}
        </p>
        {#if r.ingredientes.length}<p class="ingredientes">{r.ingredientes.join(', ')}</p>{/if}
      </div>
      <label class="cuantas"
        ><span class="sr-only">{t.cuantas}</span><input
          type="number"
          min="0.25"
          max="10"
          step="0.25"
          value={porciones[r.id] ?? 1}
          aria-label={`${t.cuantas}: ${r.nombre}`}
          oninput={(e) => (porciones[r.id] = Number(e.currentTarget.value))}
        /></label
      >
      <div class="acciones">
        <button
          type="button"
          class="primario"
          disabled={acciones.ocupado || dia !== diaLocal()}
          onclick={() =>
            void acciones.hacer(() => ipc.recetaRegistrar(r.id, porciones[r.id] ?? 1, comida))}
          >{t.registrar}</button
        ><BotonEliminar eliminar={() => ipc.recetaBorrar(r.id)} />
      </div>
    </article>
  {/each}
</section>

<style>
  .receta {
    display: grid;
    grid-template-columns: 1fr 5rem auto;
    align-items: center;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-3) 0;
    border-bottom: 1px solid color-mix(in srgb, var(--m3c-outline-variant) 55%, transparent);
  }
  .datos p {
    margin: var(--sc-espacio-1) 0 0;
  }
  .ingredientes {
    font-size: var(--sc-texto-sm);
    color: var(--m3c-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cuantas input {
    width: 100%;
    text-align: center;
  }
  .acciones {
    display: flex;
    gap: var(--sc-espacio-2);
    align-items: center;
  }
</style>
