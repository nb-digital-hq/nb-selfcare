<script lang="ts">
  import Icon from './Icon.svelte';
  import type { NombreDeIcono } from './iconos';
  let {
    titulo,
    texto,
    icono = 'lista'
  }: { titulo: string; texto?: string; icono?: NombreDeIcono } = $props();
</script>

<div class="vacio">
  <Icon nombre={icono} tamano={30} /><strong>{titulo}</strong>{#if texto}<p>{texto}</p>{/if}
</div>

<style>
  .vacio {
    display: grid;
    justify-items: center;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-7);
    text-align: center;
    color: var(--m3c-on-surface-variant);
  }
  .vacio p {
    margin: 0;
    max-width: 36rem;
  }
</style>
