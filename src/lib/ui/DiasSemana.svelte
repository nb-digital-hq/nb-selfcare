<script lang="ts">
  import { textos } from '#lib/i18n/index.ts';
  let { valor = $bindable(127) }: { valor?: number } = $props();
</script>

<fieldset class="dias">
  <legend>{textos.ui.dias}</legend>{#each Object.entries(textos.dias) as [dia, nombre]}<label
      ><input
        type="checkbox"
        checked={Boolean(valor & (1 << Number(dia)))}
        onchange={(e) =>
          (valor = e.currentTarget.checked
            ? valor | (1 << Number(dia))
            : valor & ~(1 << Number(dia)))}
      />{nombre}</label
    >{/each}
</fieldset>

<style>
  .dias {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sc-espacio-3);
    border: 0;
    padding: 0;
  }
  .dias legend {
    margin-bottom: var(--sc-espacio-2);
  }
  .dias label {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-1);
  }
</style>
