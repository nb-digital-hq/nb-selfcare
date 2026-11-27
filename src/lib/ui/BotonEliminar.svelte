<script lang="ts">
  import { textos } from '#lib/i18n/index.ts';
  import DialogoConfirmar from './DialogoConfirmar.svelte';
  import MenuAcciones from './MenuAcciones.svelte';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  let {
    eliminar,
    titulo,
    archivar
  }: { eliminar: () => Promise<unknown>; titulo?: string; archivar?: () => Promise<unknown> } =
    $props();
  let confirmar = $state(false);
  const acciones = new Acciones();
</script>

<MenuAcciones
  etiqueta={titulo ?? textos.comun.borrar}
  acciones={[
    ...(archivar
      ? [
          {
            texto: textos.comun.archivar,
            icono: 'carpeta' as const,
            accion: () => void acciones.hacer(archivar)
          }
        ]
      : []),
    {
      texto: textos.comun.borrar,
      icono: 'borrar',
      destructiva: true,
      accion: () => (confirmar = true)
    }
  ]}
/>
<DialogoConfirmar
  abierto={confirmar}
  titulo={titulo ?? textos.comun.borrar}
  mensaje={textos.comun.eliminarMensaje}
  destructivo
  ocupado={acciones.ocupado}
  oncerrar={() => (confirmar = false)}
  onconfirmar={() =>
    void acciones.hacer(async () => {
      await eliminar();
      confirmar = false;
    })}
/>
