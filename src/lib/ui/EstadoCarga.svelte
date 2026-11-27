<script lang="ts">
  import { mensajeError } from '#lib/i18n/errores.ts';
  import { textos } from '#lib/i18n/index.ts';
  import Spinner from './Spinner.svelte';
  let {
    cargando,
    fallo,
    reintentar
  }: { cargando: boolean; fallo: string | null; reintentar: () => void } = $props();
</script>

{#if fallo}<div class="error-lectura" role="alert">
    <p>{mensajeError(fallo)}</p>
    <button type="button" onclick={reintentar}>{textos.comun.reintentar}</button>
  </div>
{:else if cargando}<div class="carga">
    <Spinner etiqueta={textos.comun.cargando} /><span>{textos.comun.cargando}</span>
  </div>{/if}

<style>
  .carga,
  .error-lectura {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-4);
  }
  .error-lectura {
    color: var(--m3c-error);
  }
</style>
