<script lang="ts" module>
  /** La sección elegida sobrevive al remontaje que provoca un cambio de datos. */
  const memoria = { seccion: 'pausas' };
</script>

<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import Secciones from '#lib/ui/Secciones.svelte';
  import Cajon from '#lib/ui/Cajon.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import SesionGuiada from '#lib/ui/SesionGuiada.svelte';
  import DiasSemana from '#lib/ui/DiasSemana.svelte';
  import BotonEliminar from '#lib/ui/BotonEliminar.svelte';
  const recurso = new Recurso<ipc.ResumenCuidado>();
  const acciones = new Acciones();
  let nombre = $state('');
  let reintento = $state(0);
  let nuevoHabito = $state(false);
  let seccion = $state(memoria.seccion);
  $effect(() => {
    memoria.seccion = seccion;
  });
  const secciones = $derived([
    { id: 'pausas', titulo: textos.ui.pausas },
    { id: 'tratamientos', titulo: textos.ui.tratamientos },
    { id: 'habitos', titulo: textos.ui.habitos },
    { id: 'salud', titulo: textos.ui.signos },
    { id: 'libre', titulo: textos.ui.libre },
    { id: 'bienestar', titulo: textos.ui.bienestar }
  ]);
  let dias = $state(127);
  let respuestas = $state([3, 3, 3, 3, 3]);
  const cargar = () => recurso.cargar(ipc.cuidadoResumen);
  $effect(() => {
    void cambios.version;
    void cargar();
  });
  onDestroy(() => recurso.cancelar());
</script>

<h1>{textos.nav.cuidado}</h1>
<p class="subtitulo">{textos.ui.cuidadoSub}</p>
<Secciones
  clave="cuidado"
  opciones={secciones}
  bind:actual={seccion}
  etiqueta={textos.nav.cuidado}
/>
<div id="cuidado-contenido" role="tabpanel" aria-labelledby={`cuidado-${seccion}`}>
  {#if seccion === 'pausas'}<section class="panel">
      <h2>{textos.ui.pausas}</h2>
      <div class="dos-columnas">
        {#if recurso.datos?.modulos.includes('active_breaks')}<SesionGuiada
            tipo="pausa"
          /><SesionGuiada tipo="vista" />{/if}<SesionGuiada
          tipo="respiracion"
        />{#if recurso.datos?.modulos.includes('pelvic_floor')}<SesionGuiada tipo="pelvico" />{/if}
      </div>
    </section>
  {/if}
  <EstadoCarga cargando={recurso.cargando} fallo={recurso.fallo} reintentar={() => void cargar()} />
  {#if recurso.datos && !recurso.fallo}
    {#if seccion === 'tratamientos'}
      {#if recurso.datos.modulos.includes('treatments')}{#key reintento}{#await import('#lib/ui/cuidado/Tratamientos.svelte')}<EstadoCarga
              cargando={true}
              fallo={null}
              reintentar={() => reintento++}
            />{:then modulo}<modulo.default datos={recurso.datos} />{:catch}<EstadoCarga
              cargando={false}
              fallo="COMPONENT_UNAVAILABLE"
              reintentar={() => reintento++}
            />{/await}{/key}{/if}
    {:else if seccion === 'habitos'}
      {#if recurso.datos.modulos.includes('habits')}
        <section class="panel">
          <h2>{textos.ui.habitos}</h2>
          {#each recurso.datos.habitos as h (h.id)}<div class="fila">
              <label class="check"
                ><input
                  type="checkbox"
                  checked={h.hechoHoy}
                  disabled={acciones.ocupado || !h.tocaHoy}
                  onchange={(e) =>
                    void acciones.hacer(
                      () => ipc.habitoMarcar(h.id, e.currentTarget.checked),
                      false
                    )}
                />{h.nombre}</label
              ><BotonEliminar
                eliminar={() => ipc.habitoQuitar(h.id, true)}
                archivar={() => ipc.habitoQuitar(h.id, false)}
              />
            </div>{/each}
          <button class="primario" onclick={() => (nuevoHabito = true)}
            >{textos.ui.nuevoHabito}</button
          >
          {#if nuevoHabito}<Cajon
              titulo={textos.ui.nuevoHabito}
              icono="mas"
              oncerrar={() => (nuevoHabito = false)}
            >
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  void acciones.hacer(async () => {
                    await ipc.habitoCrear(nombre, 'spark', dias);
                    nombre = '';
                    nuevoHabito = false;
                  });
                }}
              >
                <h3>{textos.ui.nuevoHabito}</h3>
                <label
                  >{textos.ui.nombre}<input bind:value={nombre} maxlength="80" required /></label
                ><DiasSemana bind:valor={dias} /><button disabled={acciones.ocupado}
                  >{textos.comun.guardar}</button
                >
              </form></Cajon
            >{/if}
        </section>
      {/if}
    {:else if seccion === 'salud'}{#key reintento}{#await import('#lib/ui/cuidado/RegistrosSalud.svelte')}<EstadoCarga
            cargando={true}
            fallo={null}
            reintentar={() => reintento++}
          />{:then modulo}<modulo.default datos={recurso.datos} />{:catch}<EstadoCarga
            cargando={false}
            fallo="COMPONENT_UNAVAILABLE"
            reintentar={() => reintento++}
          />{/await}{/key}{:else if seccion === 'libre'}{#key reintento}{#await import('#lib/ui/cuidado/Libre.svelte')}<EstadoCarga
            cargando={true}
            fallo={null}
            reintentar={() => reintento++}
          />{:then modulo}<modulo.default datos={recurso.datos} />{:catch}<EstadoCarga
            cargando={false}
            fallo="COMPONENT_UNAVAILABLE"
            reintentar={() => reintento++}
          />{/await}{/key}{:else if seccion === 'bienestar'}
      <details class="panel">
        <summary>{textos.ui.bienestar}</summary>
        <p class="ayuda">{textos.ui.bienestarAyuda}</p>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void acciones.hacer(() => ipc.bienestarGuardar([...respuestas]));
          }}
        >
          <div class="campos">
            {#each respuestas as _, i}<label
                >{textos.ui[
                  `bienestar${i + 1}` as
                    'bienestar1' | 'bienestar2' | 'bienestar3' | 'bienestar4' | 'bienestar5'
                ]}<input type="number" bind:value={respuestas[i]} min="0" max="5" required /></label
              >{/each}
          </div>
          <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
        </form>
      </details>
    {/if}
  {/if}
</div>
