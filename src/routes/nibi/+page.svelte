<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { nombreCatalogo } from '#lib/i18n/catalogo.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import Secciones from '#lib/ui/Secciones.svelte';
  import Cajon from '#lib/ui/Cajon.svelte';
  import Icon from '#lib/ui/Icon.svelte';
  import SesionGuiada from '#lib/ui/SesionGuiada.svelte';
  const recurso = new Recurso<ipc.EstadoNibi>();
  const acciones = new Acciones();
  let reintentoMascota = $state(0);
  let nombre = $state('');
  let renombrar = $state(false);
  let filtro = $state('todos');
  const filtros = $derived([
    { id: 'todos', titulo: textos.ui.todosArticulos },
    { id: 'sombrero', titulo: textos.ui.sombreros },
    { id: 'accesorio', titulo: textos.ui.accesorios },
    { id: 'color', titulo: textos.ui.coloresNibi },
    { id: 'escenario', titulo: textos.ui.escenarios },
    { id: 'escudo', titulo: textos.ui.escudos }
  ]);
  function previsualizar(articulo: ipc.Articulo, estado: ipc.EstadoNibi): ipc.EstadoNibi {
    return {
      ...estado,
      sombrero: articulo.tipo === 'sombrero' ? articulo.id : estado.sombrero,
      accesorio: articulo.tipo === 'accesorio' ? articulo.id : estado.accesorio,
      color: articulo.tipo === 'color' ? articulo.id : estado.color,
      escenario: articulo.tipo === 'escenario' ? articulo.id : estado.escenario
    };
  }
  const cargar = () => recurso.cargar(ipc.nibiEstado);
  $effect(() => {
    void cambios.version;
    void cargar();
  });
  onDestroy(() => recurso.cancelar());
</script>

<div class="cabecera-vista">
  <div>
    <h1>{textos.nav.nibi}</h1>
    <p class="subtitulo">{textos.ui.nibiSub}</p>
  </div>
  <button onclick={() => (renombrar = true)}
    ><Icon nombre="editar" tamano={17} />{textos.ui.renombrar}</button
  >
</div>
<EstadoCarga cargando={recurso.cargando} fallo={recurso.fallo} reintentar={() => void cargar()} />
{#if recurso.datos && !recurso.fallo}{#key reintentoMascota}{#await import('#lib/ui/Nibi.svelte')}<EstadoCarga
        cargando={true}
        fallo={null}
        reintentar={() => reintentoMascota++}
      />{:then mascota}<div class="santuario">
        <section class="panel nibi">
          <mascota.default estado={recurso.datos} />
          <h2 class="nombre">{recurso.datos.nombre}</h2>
          <p class="ayuda nombre">{textos.ui.nibiSub}</p>
        </section>
        <div class="compania">
          <section class="panel">
            <div class="saldos">
              {#each [[textos.ui.monedas, recurso.datos.monedas], [textos.ui.racha, recurso.datos.racha], [textos.ui.escudos, recurso.datos.escudos]] as [titulo, valor]}<div
                >
                  <span>{titulo}</span><strong>{valor}</strong>
                </div>{/each}
            </div>
            <p class="ayuda">{textos.ui.sinPago}</p>
          </section>
          <section class="panel"><SesionGuiada tipo="respiracion" /></section>
        </div>
      </div>
      {#if renombrar}<Cajon
          titulo={textos.ui.renombrar}
          icono="editar"
          oncerrar={() => (renombrar = false)}
        >
          <form
            class="campos"
            onsubmit={(e) => {
              e.preventDefault();
              void acciones.hacer(async () => {
                await ipc.nibiRenombrar(nombre);
                nombre = '';
              });
            }}
          >
            <label>{textos.ui.nombre}<input bind:value={nombre} maxlength="24" required /></label
            ><button disabled={acciones.ocupado}>{textos.ui.renombrar}</button>
          </form></Cajon
        >{/if}
      <section class="panel">
        <h2>{textos.ui.tienda}</h2>
        <Secciones
          clave="armario"
          opciones={filtros}
          bind:actual={filtro}
          etiqueta={textos.ui.tienda}
        />
        <div
          id="armario-contenido"
          class="tienda"
          role="tabpanel"
          aria-labelledby={`armario-${filtro}`}
        >
          {#each recurso.datos.articulos.filter((a) => filtro === 'todos' || a.tipo === filtro) as a (a.id)}<div
              class="articulo"
            >
              <div class="vista-articulo">
                {#if a.tipo === 'escudo'}<Icon
                    nombre="privacidad"
                    tamano={60}
                  />{:else}<mascota.default
                    estado={previsualizar(a, recurso.datos)}
                    compacto
                  />{/if}
              </div>
              <strong>{nombreCatalogo(a.id)}</strong><span class="ayuda"
                >{a.precio} · {textos.ui.monedas}</span
              >{#if a.puesto && (a.tipo === 'sombrero' || a.tipo === 'accesorio')}<button
                  disabled={acciones.ocupado}
                  onclick={() => void acciones.hacer(() => ipc.nibiPoner(a.id))}
                  >{textos.comun.quitar}</button
                >{:else if a.puesto}<span>{textos.ui.puesto}</span
                >{:else if a.tenido && a.tipo !== 'escudo'}<button
                  type="button"
                  disabled={acciones.ocupado}
                  onclick={() => void acciones.hacer(() => ipc.nibiPoner(a.id))}
                  >{textos.ui.poner}</button
                >{:else}<button
                  type="button"
                  disabled={acciones.ocupado || recurso.datos.monedas < a.precio}
                  onclick={() => void acciones.hacer(() => ipc.nibiComprar(a.id))}
                  >{textos.ui.comprar}</button
                >{/if}
            </div>{/each}
        </div>
      </section>{:catch}<EstadoCarga
        cargando={false}
        fallo="COMPONENT_UNAVAILABLE"
        reintentar={() => reintentoMascota++}
      />{/await}{/key}{/if}

<style>
  .santuario {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: var(--sc-espacio-5);
  }
  .nibi {
    background: linear-gradient(
      160deg,
      var(--m3c-surface-container),
      var(--m3c-secondary-container)
    );
  }
  .saldos {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sc-espacio-4);
    margin-bottom: var(--sc-espacio-5);
  }
  .saldos > div {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-2);
  }
  .saldos span {
    font-size: var(--sc-texto-xs);
    color: var(--m3c-on-surface-variant);
  }
  .saldos strong {
    font-size: var(--sc-texto-2xl);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.04em;
  }
  .vista-articulo {
    min-height: 8rem;
    display: grid;
    place-items: center;
    color: var(--m3c-primary);
    background: var(--m3c-surface-container-high);
    border-radius: var(--sc-radio-md);
  }
  .vista-articulo > :global(div) {
    width: 100%;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .santuario {
    grid-template-columns: 1fr;
  }

  .nombre {
    text-align: center;
    margin-top: var(--sc-espacio-4);
  }
  .tienda {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
    gap: var(--sc-espacio-3);
  }
  .articulo {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-4);
    border: 1px solid transparent;
    background: var(--m3c-surface-container-high);
    border-radius: var(--sc-radio-md);
  }
</style>
