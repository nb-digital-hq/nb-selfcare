<script lang="ts">
  import { page } from '$app/state';
  import { DESTINOS, estaActivo } from '#lib/core/navegacion.ts';
  import { aparienciaActual } from '#lib/core/apariencia.svelte.ts';
  import { textos } from '#lib/i18n/index.ts';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';
  const grupos = $derived([
    {
      titulo: textos.nav.bienestar,
      destinos: DESTINOS.filter((d) =>
        ['hoy', 'comida', 'cuerpo', 'cuidado', 'ayuno'].includes(d.id)
      )
    },
    {
      titulo: textos.ui.espacioPersonal,
      destinos: DESTINOS.filter((d) => ['nibi', 'perfil'].includes(d.id))
    }
  ]);
  const ajustes = DESTINOS.find((d) => d.id === 'ajustes');
  if (!ajustes) throw new Error('Falta el destino de ajustes');
</script>

<nav class="lateral" aria-label={textos.nav.principal}>
  <a class="marca" href="/" aria-label={textos.app.nombre} title={textos.app.nombre}><Logo /></a>
  <div class="navegacion">
    {#each grupos as grupo}
      <section aria-label={grupo.titulo}>
        <p class="rotulo">{grupo.titulo}</p>
        {#each grupo.destinos as destino}
          <a
            class="destino"
            class:activo={estaActivo(destino, page.url.pathname)}
            href={destino.ruta}
            aria-current={estaActivo(destino, page.url.pathname) ? 'page' : undefined}
            title={destino.etiqueta}
          >
            <Icon nombre={destino.icono} tamano={19} /><span>{destino.etiqueta}</span>
          </a>
        {/each}
      </section>
    {/each}
  </div>
  <a
    class="destino ajustes"
    class:activo={estaActivo(ajustes, page.url.pathname)}
    href={ajustes.ruta}
    aria-current={estaActivo(ajustes, page.url.pathname) ? 'page' : undefined}
    title={ajustes.etiqueta}><Icon nombre="ajustes" tamano={19} /><span>{ajustes.etiqueta}</span></a
  >
  <button
    class="colapsar"
    type="button"
    onclick={() => aparienciaActual.alternarBarraLateral()}
    aria-label={aparienciaActual.barraLateralCompacta
      ? textos.nav.expandirLateral
      : textos.nav.colapsarLateral}
  >
    <Icon nombre="menu" tamano={18} /><span
      >{aparienciaActual.barraLateralCompacta
        ? textos.nav.expandirLateral
        : textos.nav.colapsarLateral}</span
    >
  </button>
</nav>

<style>
  .lateral {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--sc-espacio-4) var(--sc-espacio-3);
    gap: var(--sc-espacio-2);
  }
  .marca {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sc-espacio-1) 0 var(--sc-espacio-4);
    --logo-tamano: 5rem;
  }
  .navegacion {
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-4);
  }
  .rotulo {
    font-size: var(--sc-texto-xs);
    letter-spacing: 0.075em;
    text-transform: uppercase;
    font-weight: 650;
    color: var(--m3c-on-surface-variant);
    margin: var(--sc-espacio-2) var(--sc-espacio-4);
  }
  .destino,
  .colapsar {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-3) var(--sc-espacio-4);
    border-radius: var(--sc-radio-pleno);
    text-decoration: none;
    color: var(--m3c-on-surface-variant);
    min-height: 2.7rem;
    font-size: var(--sc-texto-sm);
    font-weight: 550;
    white-space: nowrap;
  }
  .destino:hover {
    background: var(--m3c-surface-container-high);
  }
  .destino.activo {
    background: var(--m3c-primary);
    color: var(--m3c-on-primary);
  }
  .ajustes {
    margin-top: auto;
  }
  .colapsar {
    background: none;
    border: 0;
    text-align: left;
    border-top: 1px solid var(--m3c-outline-variant);
    border-radius: 0;
    padding-top: var(--sc-espacio-4);
  }
  :global(.carcasa.compacta) .marca,
  :global(.carcasa:not([data-ancho='amplio'])) .marca {
    --logo-tamano: 2.6rem;
  }
  :global(.carcasa.compacta) span,
  :global(.carcasa:not([data-ancho='amplio'])) span,
  :global(.carcasa.compacta) .rotulo,
  :global(.carcasa:not([data-ancho='amplio'])) .rotulo {
    display: none;
  }
  :global(.carcasa.compacta) .destino,
  :global(.carcasa:not([data-ancho='amplio'])) .destino,
  :global(.carcasa.compacta) .colapsar,
  :global(.carcasa:not([data-ancho='amplio'])) .colapsar {
    justify-content: center;
    padding: var(--sc-espacio-3);
  }
</style>
