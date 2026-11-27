<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { sesionCompletar } from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  let { tipo = 'pausa' }: { tipo?: 'pausa' | 'vista' | 'respiracion' | 'pelvico' } = $props();
  let inicio = $state<number | null>(null);
  let transcurrido = $state(0);
  let reloj: ReturnType<typeof setInterval> | undefined;
  const acciones = new Acciones();
  const duracion = $derived(tipo === 'vista' ? 20 : tipo === 'pelvico' ? 90 : 120);
  const titulo = $derived(
    tipo === 'vista'
      ? textos.ui.descansoVisual
      : tipo === 'respiracion'
        ? textos.ui.respiracion
        : tipo === 'pelvico'
          ? textos.ui.sueloPelvico
          : textos.ui.pausaActiva
  );
  const guia = $derived(
    tipo === 'vista'
      ? textos.ui.vistaGuia
      : tipo === 'respiracion'
        ? textos.ui.respiracionGuia
        : tipo === 'pelvico'
          ? textos.ui.pelvicoGuia
          : textos.ui.pausaGuia
  );
  function terminarReloj() {
    if (reloj !== undefined) clearInterval(reloj);
    reloj = undefined;
  }
  function empezar() {
    terminarReloj();
    inicio = performance.now();
    transcurrido = 0;
    reloj = setInterval(() => {
      if (inicio !== null)
        transcurrido = Math.min(duracion, Math.floor((performance.now() - inicio) / 1000));
      if (transcurrido >= duracion) terminarReloj();
    }, 250);
  }
  onDestroy(terminarReloj);
</script>

<div class="guia">
  <h3>{titulo}</h3>
  <p class="ayuda">{guia}</p>
  {#if inicio === null}<button type="button" onclick={empezar}>{textos.comun.iniciar}</button
    >{:else}<p role="timer" aria-label={textos.ui.restante}>
      {Math.max(0, duracion - transcurrido)} s
    </p>
    <progress max={duracion} value={transcurrido} aria-label={titulo}
    ></progress>{#if tipo === 'respiracion' && transcurrido < duracion}<p>
        {transcurrido % 10 < 4 ? textos.ui.inhalar : textos.ui.exhalar}
      </p>{/if}{#if transcurrido >= duracion}<p>{textos.ui.sesionCompleta}</p>
      <button
        type="button"
        disabled={acciones.ocupado}
        onclick={() =>
          void acciones.hacer(async () => {
            await sesionCompletar(
              tipo === 'respiracion'
                ? 'breathing'
                : tipo === 'pelvico'
                  ? 'pelvic_floor'
                  : 'active_break',
              tipo === 'vista'
                ? 'eyes'
                : tipo === 'pelvico'
                  ? 'level_1'
                  : tipo === 'respiracion'
                    ? 'calm'
                    : 'desk',
              duracion
            );
            inicio = null;
            transcurrido = 0;
          })}>{textos.comun.guardar}</button
      >{/if}<button
      type="button"
      onclick={() => {
        terminarReloj();
        inicio = null;
      }}>{textos.comun.cancelar}</button
    >{/if}
</div>

<style>
  .guia {
    padding: var(--sc-espacio-4);
    background: var(--m3c-surface-container-high);
    border-radius: var(--sc-radio-md);
  }
  .guia progress {
    width: 100%;
  }
  h3 {
    margin-top: 0;
  }
</style>
