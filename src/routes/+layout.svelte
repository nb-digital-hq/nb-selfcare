<script lang="ts">
  import '../fuentes.css';
  import '../app.css';
  import { onMount, tick } from 'svelte';
  import { aparienciaActual } from '#lib/core/apariencia.svelte.ts';
  import { mostrarVentanaTrasApariencia } from '#lib/core/arranque.ts';
  import {
    arranqueEnSegundoPlano,
    mostrarVentanaPrincipal,
    registrarVentanaNoVisible
  } from '#lib/core/ipc/index.ts';
  import AppShell from '#lib/ui/AppShell.svelte';
  let { children } = $props();
  onMount(() => {
    aparienciaActual.aplicarCache(document.documentElement);
    const lista = aparienciaActual.cargar(document.documentElement);
    void mostrarVentanaTrasApariencia(lista, async () => {
      if (await arranqueEnSegundoPlano()) return;
      await tick();
      await mostrarVentanaPrincipal();
    }).catch(() => {
      void registrarVentanaNoVisible().catch(() => {});
    });
  });
</script>

<AppShell>{@render children()}</AppShell>
