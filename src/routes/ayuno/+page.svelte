<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { numero } from '#lib/core/dia.ts';
  import Anillo from '#lib/ui/Anillo.svelte';
  import Metrica from '#lib/ui/Metrica.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import BotonEliminar from '#lib/ui/BotonEliminar.svelte';
  const recurso = new Recurso<ipc.EstadoAyuno>();
  const acciones = new Acciones();
  let horas = $state(16);
  let hace = $state(0);
  let aviso = $state<ipc.Aviso | null>(null);
  let objetivoRevisado = $state<number | null>(null);
  let reloj = $state(Date.now());
  const cargar = () => recurso.cargar(ipc.ayunoEstado);
  $effect(() => {
    void cambios.version;
    void cargar();
  });
  onDestroy(() => recurso.cancelar());
  onMount(() => {
    const t = setInterval(() => (reloj = Date.now()), 1000);
    return () => clearInterval(t);
  });
  // Cada lectura de Rust renueva la referencia para no contar dos veces el tiempo.
  const transcurrido = $derived(
    Math.max(0, (recurso.datos?.enCurso?.horas ?? 0) + (reloj - recurso.actualizadoEn) / 3600000)
  );
  async function comenzar(confirmado = false) {
    const alerta = await ipc.ayunoComprobar(horas);
    if (alerta && !(confirmado && objetivoRevisado === horas && aviso?.motivo === alerta.motivo)) {
      aviso = alerta;
      objetivoRevisado = horas;
      return;
    }
    await ipc.ayunoEmpezar(`${horas} h`, horas, hace);
    aviso = null;
    objetivoRevisado = null;
  }
</script>

<h1>{textos.nav.ayuno}</h1>
<p class="subtitulo">{textos.ui.ayunoSub}</p>
<EstadoCarga cargando={recurso.cargando} fallo={recurso.fallo} reintentar={() => void cargar()} />
{#if recurso.datos && !recurso.fallo}
  {#if !recurso.datos.activo}<section class="panel">
      <p class="ayuda">{textos.comun.estimacion}</p>
      <button
        type="button"
        disabled={acciones.ocupado}
        onclick={() => void acciones.hacer(() => ipc.perfilCambiar({ ayunoActivo: true }))}
        >{textos.ui.habilitarAyuno}</button
      >
    </section>
  {:else if recurso.datos.enCurso}<section class="panel cronometro">
      <h2>{recurso.datos.enCurso.protocolo}</h2>
      <Anillo
        valor={transcurrido}
        maximo={recurso.datos.enCurso.horasMeta}
        etiqueta={textos.nav.ayuno}
        texto={`${Math.floor(transcurrido).toString().padStart(2, '0')}:${Math.floor(
          (transcurrido % 1) * 60
        )
          .toString()
          .padStart(2, '0')}:${Math.floor((transcurrido * 3600) % 60)
          .toString()
          .padStart(2, '0')}`}
        unidad={textos.ui.tiempoTranscurrido}
        animar={false}
      />
      <p>{textos.ui.objetivo}: {recurso.datos.enCurso.horasMeta} h</p>
      <button
        type="button"
        class="primario"
        disabled={acciones.ocupado}
        onclick={() => void acciones.hacer(ipc.ayunoTerminar)}>{textos.comun.terminar}</button
      >
    </section>
    <form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => ipc.ayunoAjustar(hace));
      }}
    >
      <label
        >{textos.ui.haceMinutos}<input
          type="number"
          bind:value={hace}
          min="0"
          max="2880"
          required
        /></label
      ><button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
    </form>
  {:else}<form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => comenzar(), false);
      }}
    >
      <h2>{textos.ui.elegirDuracion}</h2>
      <div class="acciones">
        {#each [12, 14, 16] as duracion}<button
            type="button"
            class:primario={horas === duracion}
            aria-pressed={horas === duracion}
            onclick={() => {
              horas = duracion;
              aviso = null;
              objetivoRevisado = null;
            }}>{duracion} {textos.unidades.h}</button
          >{/each}
      </div>
      <div class="campos">
        <label
          >{textos.ui.horasMeta}<input
            type="number"
            bind:value={horas}
            min="1"
            max="72"
            oninput={() => {
              aviso = null;
              objetivoRevisado = null;
            }}
            required
          /></label
        ><label
          >{textos.ui.haceMinutos}<input
            type="number"
            bind:value={hace}
            min="0"
            max="2880"
            required
          /></label
        >
      </div>
      <button class="primario" disabled={acciones.ocupado}>{textos.comun.iniciar}</button
      >{#if aviso}<div class="resultado" role="alert">
          <strong>{textos.ui.ayunoAviso}</strong>
          <p>
            {aviso.motivo === 'tca'
              ? textos.ui.ayunoTca
              : aviso.motivo === 'glucosa'
                ? textos.ui.ayunoGlucosa
                : textos.ui.ayunoFrecuencia}
          </p>
          <div class="acciones">
            {#if aviso.alternativa !== null}<button
                type="button"
                onclick={() => {
                  if (aviso?.alternativa !== null && aviso?.alternativa !== undefined)
                    horas = aviso.alternativa;
                  aviso = null;
                  objetivoRevisado = null;
                }}>{textos.ui.alternativa}</button
              >{/if}<button
              type="button"
              disabled={acciones.ocupado}
              onclick={() => void acciones.hacer(() => comenzar(true))}
              >{textos.comun.confirmar}</button
            ><button type="button" onclick={() => (aviso = null)}>{textos.comun.cancelar}</button>
          </div>
        </div>{/if}
    </form>{/if}
  <div class="metricas resumen-ayunos">
    <Metrica titulo={textos.ui.frecuenciaSemanal} valor={recurso.datos.rachaSemanal} /><Metrica
      titulo={textos.ui.duracionMedia}
      valor={recurso.datos.mediaHoras}
      unidad={textos.unidades.h}
    />
  </div>
  <section class="panel">
    <h2>{textos.ui.historial}</h2>
    {#each recurso.datos.historial as a (a.id)}<div class="fila">
        <span>{a.dia} · {a.protocolo} · {numero(a.horas, 1)} h</span><BotonEliminar
          eliminar={() => ipc.ayunoBorrar(a.id)}
        />
      </div>{/each}
  </section>
  <p class="ayuda">{textos.comun.estimacion}</p>
{/if}

<style>
  .cronometro {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sc-espacio-4);
    padding: var(--sc-espacio-7);
  }
  .cronometro {
    --anillo-tamano: 18rem;
    --anillo-texto: var(--sc-texto-3xl);
    background: linear-gradient(
      160deg,
      var(--m3c-surface-container),
      var(--m3c-secondary-container)
    );
  }
  .resumen-ayunos {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
</style>
