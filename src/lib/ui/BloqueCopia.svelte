<script lang="ts">
  import { tick } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import {
    copiaElegir,
    copiaExportar,
    copiaRestaurar,
    type Inspeccion
  } from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import DialogoConfirmar from './DialogoConfirmar.svelte';
  const acciones = new Acciones();
  let copia = $state<Inspeccion | null>(null);
  let confirmar = $state(false);
  let botonElegir: HTMLButtonElement | undefined = $state();

  async function restaurarElegida() {
    await acciones.hacer(async () => {
      try {
        await copiaRestaurar();
      } finally {
        // La selección es de un solo uso y el aviso debe quedar fuera del modal.
        copia = null;
        confirmar = false;
        await tick();
      }
    });
    // Acciones habilita el selector al terminar; antes de ese cambio no acepta foco.
    await tick();
    if (botonElegir?.isConnected) botonElegir.focus();
  }
</script>

<section class="panel">
  <h2>{textos.ui.copias}</h2>
  <p class="ayuda">{textos.ui.copiaPrivada}</p>
  <div class="acciones">
    <button
      type="button"
      disabled={acciones.ocupado}
      onclick={() =>
        void acciones.hacer(async () => {
          await copiaExportar();
        }, false)}>{textos.ui.exportar}</button
    ><button
      bind:this={botonElegir}
      type="button"
      disabled={acciones.ocupado}
      onclick={() =>
        void acciones.hacer(async () => {
          copia = await copiaElegir();
        }, false)}>{textos.ui.elegirCopia}</button
    >
  </div>
  {#if copia}<div class="resultado">
      <strong>{copia.perfil ?? textos.app.nombre}</strong>
      <p>
        {textos.ui.origen}: {copia.origen === 'mobile' ? textos.ui.mobile : textos.ui.desktop} · {textos
          .ui.registros}: {copia.registros}
      </p>
      <p>{textos.ui.restaurarAviso}</p>
      <button
        type="button"
        class="peligro"
        disabled={acciones.ocupado}
        onclick={() => (confirmar = true)}>{textos.ui.restaurar}</button
      >
    </div>{/if}
</section>
<DialogoConfirmar
  abierto={confirmar}
  titulo={textos.ui.restaurar}
  mensaje={textos.ui.restaurarAviso}
  textoConfirmar={textos.ui.restaurar}
  destructivo
  ocupado={acciones.ocupado}
  oncerrar={() => (confirmar = false)}
  onconfirmar={() => void restaurarElegida()}
/>
