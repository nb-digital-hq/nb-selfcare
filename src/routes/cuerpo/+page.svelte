<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Recurso, Acciones } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { numero } from '#lib/core/dia.ts';
  import { notificar } from '#lib/core/notificaciones.svelte.ts';
  import Cajon from '#lib/ui/Cajon.svelte';
  import Icon from '#lib/ui/Icon.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import EstadoVacio from '#lib/ui/EstadoVacio.svelte';
  import Metrica from '#lib/ui/Metrica.svelte';
  import BotonEliminar from '#lib/ui/BotonEliminar.svelte';
  import DialogoConfirmar from '#lib/ui/DialogoConfirmar.svelte';
  const recurso = new Recurso<ipc.ResumenCuerpo>();
  const acciones = new Acciones();
  let registro = $state<'peso' | 'cinta' | 'entrenamiento' | null>(null);
  let peso = $state<number | undefined>();
  let confirmarPeso = $state<number | null>(null);
  let cintura = $state(80);
  let cuello = $state(35);
  let cadera = $state<number | undefined>();
  let categoria = $state<ipc.Categoria>('cardio');
  let intensidad = $state<ipc.Intensidad>('moderada');
  let minutos = $state(30);
  let resultado = $state<ipc.ResultadoPesaje | null>(null);
  const datos = $derived(recurso.datos);
  const cargar = () => recurso.cargar(ipc.cuerpoResumen);
  $effect(() => {
    void cambios.version;
    void cargar();
  });
  onDestroy(() => recurso.cancelar());
  const puntos = $derived.by(() => {
    const serie = datos?.serie ?? [];
    if (!serie.length) return '';
    const lo = Math.min(...serie.map((p) => p.tendenciaKg)) - 1;
    const hi = Math.max(...serie.map((p) => p.tendenciaKg)) + 1;
    return serie
      .map(
        (p, i) =>
          `${20 + (i * 560) / Math.max(1, serie.length - 1)},${150 - ((p.tendenciaKg - lo) / (hi - lo)) * 130}`
      )
      .join(' ');
  });
  async function guardarPesaje(valor: number) {
    resultado = await ipc.cuerpoPesar(valor);
    confirmarPeso = null;
    peso = undefined;
    registro = null;
  }
</script>

<div class="cabecera-vista">
  <div>
    <h1>{textos.nav.cuerpo}</h1>
    <p class="subtitulo">{textos.ui.cuerpoSub}</p>
  </div>
  <div class="acciones">
    <button onclick={() => (registro = 'cinta')}>{textos.ui.cinta}</button><button
      onclick={() => (registro = 'entrenamiento')}>{textos.ui.entrenamiento}</button
    ><button class="primario" onclick={() => (registro = 'peso')}
      ><Icon nombre="mas" tamano={17} />{textos.ui.peso}</button
    >
  </div>
</div>
<EstadoCarga cargando={recurso.cargando} fallo={recurso.fallo} reintentar={() => void cargar()} />
{#if datos && !recurso.fallo}
  <div class="metricas">
    <Metrica titulo={textos.ui.peso} valor={datos.ultimo?.pesoKg ?? null} unidad="kg" /><Metrica
      titulo={textos.ui.tendencia}
      valor={datos.ultimo?.tendenciaKg ?? null}
      unidad="kg"
    /><Metrica titulo={textos.ui.objetivo} valor={datos.pesoMetaKg} unidad="kg" /><Metrica
      titulo={textos.ui.imc}
      valor={datos.ultimo?.imc ?? null}
    />
  </div>
  <section class="panel">
    <h2>{textos.ui.tendencia}</h2>
    {#if datos.serie.length}<svg
        class="grafica"
        viewBox="0 0 600 175"
        role="img"
        aria-label={textos.ui.tendencia}
        ><line x1="20" y1="155" x2="580" y2="155" /><polyline points={puntos} /></svg
      >
      <div class="extremos">
        <span>{datos.serie[0]?.dia}</span><span>{datos.serie.at(-1)?.dia}</span>
      </div>{:else}<EstadoVacio titulo={textos.comun.vacio} />{/if}
    <p class="ayuda">{textos.comun.estimacion}</p>
  </section>
{/if}

<section class="panel">
  <h2>{textos.ui.entrenamiento}</h2>

  {#each datos?.entrenos ?? [] as e (e.id)}<div class="fila">
      <span
        >{textos.categorias[e.categoria]} · {textos.intensidades[e.intensidad]} · {e.minutos}
        {textos.unidades.min} · {numero(e.kcal)}
        {textos.unidades.kcal}</span
      ><BotonEliminar eliminar={() => ipc.cuerpoBorrarEntreno(e.id)} />
    </div>{/each}
</section>
<section class="panel">
  <h2>{textos.ui.pesajes}</h2>
  <div class="tabla-contenedor">
    <table class="tabla">
      <thead
        ><tr
          ><th>{textos.ui.fecha}</th><th>{textos.ui.peso}</th><th>{textos.ui.tendencia}</th><th
            ><span class="sr-only">{textos.comun.borrar}</span></th
          ></tr
        ></thead
      ><tbody
        >{#each datos?.historial ?? [] as p (p.id)}<tr
            ><td>{p.dia} {p.hora}</td><td>{numero(p.pesoKg, 1)}</td><td
              >{numero(p.tendenciaKg, 1)}</td
            ><td><BotonEliminar eliminar={() => ipc.cuerpoBorrar(p.id)} /></td></tr
          >{/each}</tbody
      >
    </table>
  </div>
</section>
{#if datos}<section class="panel">
    <h2>{textos.ui.hitos}</h2>
    <div class="acciones">
      {#each datos.hitos.filter((h) => h.logrado) as h}<span
          >{numero(h.pesoKg, 1)} {textos.unidades.kg} · {h.dia}</span
        >{/each}
    </div>
    <button
      type="button"
      disabled={acciones.ocupado}
      onclick={() =>
        void acciones.hacer(async () => {
          const c = await ipc.cuerpoCalibrar();
          notificar.informacion(c.calibro ? textos.ui.calibrado : textos.ui.faltanDatos);
        }, false)}>{textos.ui.calibrar}</button
    >
  </section>{/if}
{#if registro === 'peso'}<Cajon
    titulo={textos.ui.peso}
    icono="cuerpo"
    oncerrar={() => (registro = null)}
    ><form
      class="registro-formulario"
      onsubmit={(e) => {
        e.preventDefault();
        if (peso !== undefined) {
          const valor = peso;
          void acciones.hacer(async () => {
            const ajeno = await ipc.cuerpoPareceDeOtraPersona(valor);
            if (ajeno !== null) confirmarPeso = valor;
            else await guardarPesaje(valor);
          }, false);
        }
      }}
    >
      <h2>{textos.ui.peso}</h2>
      <label
        >{textos.ui.peso}<input
          type="number"
          bind:value={peso}
          min="25"
          max="350"
          step="0.1"
          required
        /></label
      ><button class="primario" disabled={acciones.ocupado}>{textos.comun.registrar}</button
      >{#if resultado?.falsaSubida}<p class="ayuda">{textos.ui.falsaSubida}</p>{/if}
    </form></Cajon
  >{/if}
{#if registro === 'cinta'}<Cajon
    titulo={textos.ui.cinta}
    icono="cuerpo"
    oncerrar={() => (registro = null)}
    ><form
      class="registro-formulario"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() =>
          ipc.cuerpoCinta({ cinturaCm: cintura, cuelloCm: cuello, caderaCm: cadera ?? null })
        );
      }}
    >
      <h2>{textos.ui.cinta}</h2>
      <div class="campos">
        <label
          >{textos.ui.cintura}<input
            type="number"
            bind:value={cintura}
            min="30"
            max="250"
            step="0.1"
            required
          /></label
        ><label
          >{textos.ui.cuello}<input
            type="number"
            bind:value={cuello}
            min="15"
            max="100"
            step="0.1"
            required
          /></label
        ><label
          >{textos.ui.cadera}<input
            type="number"
            bind:value={cadera}
            min="30"
            max="250"
            step="0.1"
          /></label
        >
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>
    </form></Cajon
  >{/if}
{#if registro === 'entrenamiento'}<Cajon
    titulo={textos.ui.entrenamiento}
    icono="mas"
    oncerrar={() => (registro = null)}
    ><form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => ipc.cuerpoEntrenar(categoria, intensidad, minutos));
      }}
    >
      <div class="campos">
        <MenuSelector
          rotulo={textos.ui.categoria}
          bind:valor={categoria}
          opciones={Object.entries(textos.categorias).map(([valor, etiqueta]) => ({
            valor: valor as typeof categoria,
            etiqueta
          }))}
        /><MenuSelector
          rotulo={textos.ui.intensidad}
          bind:valor={intensidad}
          opciones={Object.entries(textos.intensidades).map(([valor, etiqueta]) => ({
            valor: valor as typeof intensidad,
            etiqueta
          }))}
        /><label
          >{textos.ui.minutos}<input
            type="number"
            bind:value={minutos}
            min="1"
            max="360"
            required
          /></label
        >
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>
    </form></Cajon
  >{/if}
<DialogoConfirmar
  abierto={confirmarPeso !== null}
  titulo={textos.ui.peso}
  mensaje={textos.ui.pesoAjeno}
  ocupado={acciones.ocupado}
  oncerrar={() => (confirmarPeso = null)}
  onconfirmar={() =>
    void acciones.hacer(async () => {
      if (confirmarPeso !== null) await guardarPesaje(confirmarPeso);
    })}
/>

<style>
  .grafica {
    width: 100%;
    max-height: 17rem;
  }
  .grafica line {
    stroke: var(--m3c-outline-variant);
  }
  .grafica polyline {
    fill: none;
    stroke: var(--m3c-primary);
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .extremos {
    display: flex;
    justify-content: space-between;
    color: var(--m3c-on-surface-variant);
    font-size: var(--sc-texto-xs);
  }
</style>
