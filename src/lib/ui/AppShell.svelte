<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { aparienciaActual } from '#lib/core/apariencia.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { resumen, actualizarResumen } from '#lib/core/resumen.svelte.ts';
  import { escuchar, type AvisoEscritorio } from '#lib/core/ipc/index.ts';
  import { rutaGuardadaValida, rutaDeArranque } from '#lib/core/navegacion.ts';
  import { notificar } from '#lib/core/notificaciones.svelte.ts';
  import { textos } from '#lib/i18n/index.ts';
  import { clasificarAncho } from './anchos';
  import NavLateral from './NavLateral.svelte';
  import Avisos from './Avisos.svelte';
  import EstadoCarga from './EstadoCarga.svelte';
  import Icon from './Icon.svelte';
  import { buscarAlimentos } from '#lib/core/busqueda.svelte.ts';

  let { children }: { children: Snippet } = $props();
  let ancho = $state(1280);
  let consulta = $state('');
  let rapida = $state(false);
  let restauraciones = $state(0);
  let navegacionLista = $state(false);
  $effect(() => {
    void cambios.version;
    void actualizarResumen();
  });
  onMount(() => {
    let viva = true;
    const cancelaciones: (() => void)[] = [];
    const suscribir = <T,>(evento: string, funcion: (carga: T) => void) => {
      void escuchar<T>(evento, funcion)
        .then((cancelar) => {
          if (viva) cancelaciones.push(cancelar);
          else cancelar();
        })
        .catch(() => {});
    };
    suscribir<void>('datos:cambio', () => {
      void Promise.all([
        actualizarResumen(),
        aparienciaActual.cargar(document.documentElement)
      ]).then(() => {
        if (viva && !resumen.fallo) restauraciones += 1;
      });
    });
    suscribir<AvisoEscritorio>('avisos:aviso', (aviso) => {
      if (aviso.tipo === 'toma') notificar.informacion(textos.ui.tomas);
      else
        notificar.informacion(
          aviso.tipo === 'vista'
            ? textos.ui.descansoVisual
            : aviso.tipo === 'agua'
              ? textos.ui.agua
              : textos.ui.pausaActiva
        );
    });
    suscribir<string>('avisos:empezar', () => {
      void goto('/cuidado');
    });
    void (async () => {
      try {
        const ruta = rutaDeArranque(page.url.pathname, localStorage.getItem('sc:ultima-ruta'));
        if (ruta) await goto(ruta, { replaceState: true });
      } catch {
        /* La caché no impide abrir Hoy. */
      } finally {
        navegacionLista = true;
      }
    })();
    return () => {
      viva = false;
      for (const cancelar of cancelaciones) cancelar();
      resumen.cancelar();
    };
  });
  $effect(() => {
    if (!navegacionLista) return;
    const ruta = page.url.pathname || '/';
    if (rutaGuardadaValida(ruta)) {
      try {
        localStorage.setItem('sc:ultima-ruta', ruta);
      } catch {
        /* La navegación no depende de la caché. */
      }
    }
  });
</script>

<svelte:window
  onfocus={() => cambios.avisar()}
  onkeydown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k' && resumen.datos?.perfil) {
      e.preventDefault();
      rapida = true;
    }
  }}
/>
{#if rapida}{#await import('./EntradaRapida.svelte') then modulo}<modulo.default
      oncerrar={() => (rapida = false)}
    />{/await}{/if}
<a class="saltar" href="#contenido">{textos.ui.irContenido}</a>
<div
  class="carcasa"
  class:compacta={aparienciaActual.barraLateralCompacta}
  data-ancho={clasificarAncho(ancho)}
  bind:clientWidth={ancho}
>
  <aside aria-label={textos.nav.principal}><NavLateral /></aside>
  <header class="superior">
    <form
      class="busqueda-global"
      role="search"
      onsubmit={(e) => {
        e.preventDefault();
        if (buscarAlimentos(consulta)) void goto('/comida');
      }}
    >
      <Icon nombre="buscar" tamano={19} /><input
        type="search"
        bind:value={consulta}
        maxlength="120"
        placeholder={textos.ui.buscarSuite}
        aria-label={textos.ui.buscarSuite}
      />
    </form>
    <a
      class="chip-barra"
      href="/ajustes?seccion=telefono"
      aria-label={textos.ui.telefono}
      title={textos.ui.telefono}
      ><Icon nombre="telefono" tamano={17} /><span>{textos.ui.telefono}</span></a
    >
    <button
      type="button"
      class="registro-rapido"
      aria-label={textos.ui.registroRapido}
      aria-keyshortcuts="Control+K"
      title={textos.ui.atajoRapido}
      onclick={() => (rapida = true)}
      ><Icon nombre="mas" tamano={18} /><span>{textos.ui.registroRapido}</span></button
    >
  </header>
  <div class="lienzo">
    <main id="contenido" tabindex="-1">
      <EstadoCarga
        cargando={resumen.cargando && !resumen.datos}
        fallo={resumen.fallo}
        reintentar={() => void actualizarResumen()}
      />
      {#if resumen.datos?.perfil}{#key restauraciones}{@render children()}{/key}{:else if resumen.datos && !resumen.fallo}{#await import('./Bienvenida.svelte')}
          <EstadoCarga cargando={true} fallo={null} reintentar={() => {}} />
        {:then m}<m.default />
        {/await}{/if}
    </main>
  </div>
</div>
<Avisos />

<style>
  .carcasa {
    display: grid;
    grid-template-columns: var(--sc-ancho-lateral) minmax(0, 1fr);
    grid-template-rows: var(--sc-alto-superior) minmax(0, 1fr);
    height: 100dvh;
    background: var(--m3c-surface);
  }
  .carcasa aside {
    grid-row: 1/-1;
    min-height: 0;
  }
  .superior {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sc-espacio-4);
    padding: var(--sc-espacio-2) var(--sc-espacio-5);
  }
  .busqueda-global {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    flex: 1;
    min-width: 0;
    max-width: 48rem;
    padding: 0 var(--sc-espacio-4);
    border-radius: var(--sc-radio-pleno);
    background: var(--m3c-surface-container);
    color: var(--m3c-on-surface-variant);
  }
  .busqueda-global input {
    width: 100%;
    background: transparent;
    border: 0;
    min-height: 2.7rem;
    padding: var(--sc-espacio-2) 0;
    outline-offset: 0;
  }
  .registro-rapido {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-2);
    border-radius: var(--sc-radio-pleno);
    padding: var(--sc-espacio-3) var(--sc-espacio-4);
    background: var(--m3c-primary);
    color: var(--m3c-on-primary);
    font-size: var(--sc-texto-sm);
    font-weight: 650;
    white-space: nowrap;
    text-decoration: none;
  }
  .registro-rapido {
    border: none;
    min-height: 0;
    cursor: pointer;
  }
  .chip-barra {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-2);
    border-radius: var(--sc-radio-pleno);
    padding: var(--sc-espacio-3) var(--sc-espacio-4);
    border: 1px solid var(--m3c-outline-variant);
    background: var(--m3c-surface-container);
    color: var(--m3c-on-surface);
    font-size: var(--sc-texto-sm);
    font-weight: 600;
    white-space: nowrap;
    text-decoration: none;
  }
  :global(.carcasa[data-ancho='compacto']) .chip-barra span,
  :global(.carcasa[data-ancho='compacto']) .registro-rapido span {
    display: none;
  }
  .lienzo {
    min-height: 0;
    overflow: hidden;
    margin: 0 var(--sc-espacio-4) var(--sc-espacio-4) 0;
    border-radius: var(--sc-radio-xl);
    background: var(--m3c-surface-container-low);
  }
  main {
    height: 100%;
    overflow: auto;
    padding: var(--sc-espacio-5);
  }
  .compacta,
  .carcasa:not([data-ancho='amplio']) {
    grid-template-columns: var(--sc-ancho-riel) minmax(0, 1fr);
  }
  .carcasa[data-ancho='compacto'] main {
    padding: var(--sc-espacio-4);
  }
  .saltar {
    position: fixed;
    top: -10rem;
    left: 1rem;
    z-index: 100;
    padding: 1rem;
    background: var(--m3c-primary);
    color: var(--m3c-on-primary);
  }
  .saltar:focus {
    top: 1rem;
  }
</style>
