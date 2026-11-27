<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { textos, IDIOMAS, NOMBRE_DEL_IDIOMA } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { aparienciaActual } from '#lib/core/apariencia.svelte.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { minutos, hora } from '#lib/core/dia.ts';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import DiasSemana from '#lib/ui/DiasSemana.svelte';
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import Icon from '#lib/ui/Icon.svelte';
  import { grupoDeTema } from '#lib/ui/tema.ts';
  import type { TemaDelCatalogo } from '#lib/ui/temas.ts';
  let temas = $state<readonly TemaDelCatalogo[]>([]);
  const acciones = new Acciones();
  const configuracion = new Recurso<ipc.AjustesAvisos>();
  const red = new Recurso<ipc.PoliticaDeRed>();
  let borrador = $state<ipc.AjustesAvisos | null>(null);
  let autoinicio = $state(false);
  let inicio = $state('09:00');
  let fin = $state('18:00');
  let escala = $state(100);
  let version = $state('');
  let gestionada = $state(false);
  let actualizacion = $state<ipc.Actualizacion | null>(null);
  let buscada = $state(false);
  let seccion = $state(
    (typeof location !== 'undefined' && new URLSearchParams(location.search).get('seccion')) ||
      'apariencia'
  );
  let reintentoCopia = $state(0);
  let reintentoTelefono = $state(0);
  const apartados = $derived([
    { id: 'apariencia', titulo: textos.ui.apariencia, icono: 'ajustes' },
    { id: 'avisos', titulo: textos.ui.avisos, icono: 'reloj' },
    { id: 'telefono', titulo: textos.ui.telefono, icono: 'telefono' },
    { id: 'copias', titulo: textos.ui.copias, icono: 'descargar' },
    { id: 'red', titulo: textos.ui.red, icono: 'privacidad' },
    { id: 'actualizaciones', titulo: textos.ui.actualizaciones, icono: 'reintentar' },
    { id: 'acerca', titulo: textos.ui.acerca, icono: 'informacion' }
  ] as const);
  function navegarApartados(e: KeyboardEvent, indice: number) {
    let siguiente = indice;
    if (e.key === 'ArrowDown') siguiente = (indice + 1) % apartados.length;
    else if (e.key === 'ArrowUp') siguiente = (indice + apartados.length - 1) % apartados.length;
    else if (e.key === 'Home') siguiente = 0;
    else if (e.key === 'End') siguiente = apartados.length - 1;
    else return;
    e.preventDefault();
    const apartado = apartados[siguiente];
    if (!apartado) return;
    seccion = apartado.id;
    document.getElementById(`ajuste-${seccion}`)?.focus();
  }
  const cargar = () =>
    configuracion.cargar(ipc.avisosAjustes).then(() => {
      if (borrador === null && configuracion.datos) {
        borrador = structuredClone(configuracion.datos);
        inicio = hora(borrador.jornadaDesde);
        fin = hora(borrador.jornadaHasta);
      }
    });
  $effect(() => {
    void cambios.version;
    void cargar();
    void red.cargar(ipc.politicaRed);
  });
  onMount(() => {
    void import('#lib/ui/temas.ts').then((m) => {
      temas = m.TEMAS;
    });
    escala = aparienciaActual.actual.escala;
    void acciones.hacer(async () => {
      autoinicio = await ipc.inicioConSistema(null);
      const salud = await ipc.salud();
      version = salud.version;
      gestionada = salud.gestionadaPorLauncher;
    }, false);
  });
  onDestroy(() => {
    configuracion.cancelar();
    red.cancelar();
  });
  async function politica(servicio: ipc.Servicio | null, activo: boolean) {
    const anterior = red.datos;
    if (!anterior) return;
    const politica: ipc.PoliticaDeRed =
      servicio === null
        ? { ...anterior, general: activo }
        : {
            ...anterior,
            servicios: activo
              ? [...new Set([...anterior.servicios, servicio])]
              : anterior.servicios.filter((s) => s !== servicio)
          };
    await ipc.cambiarPoliticaRed(politica);
    await red.cargar(ipc.politicaRed);
  }
</script>

<div class="ajustes-distribucion">
  <div class="apartados" role="tablist" aria-label={textos.nav.ajustes} aria-orientation="vertical">
    {#each apartados as apartado, indice}<button
        id={`ajuste-${apartado.id}`}
        role="tab"
        type="button"
        aria-selected={seccion === apartado.id}
        aria-controls={`contenido-${apartado.id}`}
        tabindex={seccion === apartado.id ? 0 : -1}
        class:activo={seccion === apartado.id}
        onclick={() => (seccion = apartado.id)}
        onkeydown={(e) => navegarApartados(e, indice)}
        ><Icon nombre={apartado.icono} tamano={18} /><span>{apartado.titulo}</span></button
      >{/each}
  </div>
  <div
    class="ajustes-contenido"
    role="tabpanel"
    id={`contenido-${seccion}`}
    aria-labelledby={`ajuste-${seccion}`}
    tabindex="0"
  >
    <h1>{textos.nav.ajustes} · {apartados.find((a) => a.id === seccion)?.titulo}</h1>
    {#if seccion === 'apariencia'}<section class="panel apariencia">
        <div class="fila-ajuste">
          <div><h2>{textos.ui.idioma}</h2></div>
          <div class="idiomas" role="group" aria-label={textos.ui.idioma}>
            {#each IDIOMAS as i}<button
                type="button"
                class:seleccionado={(aparienciaActual.actual.idioma ?? 'es') === i}
                aria-pressed={(aparienciaActual.actual.idioma ?? 'es') === i}
                disabled={acciones.ocupado}
                onclick={() =>
                  void acciones.hacer(() => aparienciaActual.cambiar('idioma', i), false)}
                >{NOMBRE_DEL_IDIOMA[i]}</button
              >{/each}
          </div>
        </div>
        <div class="campos tipografia">
          <MenuSelector
            rotulo={textos.ui.tipografia}
            valor={aparienciaActual.actual.tipografia}
            opciones={[
              'Inter',
              'Fira Sans',
              'IBM Plex Sans',
              'Atkinson Hyperlegible',
              'Source Sans 3'
            ].map((f) => ({ valor: f, etiqueta: f }))}
            controlado
            deshabilitado={acciones.ocupado}
            onchange={(valor) =>
              void acciones.hacer(() => aparienciaActual.cambiar('tipografia', valor), false)}
          />
          <form
            class="escala"
            onsubmit={(e) => {
              e.preventDefault();
              void acciones.hacer(() => aparienciaActual.cambiar('escala', escala), false);
            }}
          >
            <label
              >{textos.ui.escalaUi}<input
                type="number"
                bind:value={escala}
                min="75"
                max="150"
                step="5"
                required
              /></label
            ><button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
          </form>
        </div>
        {#each ['oscuros', 'claros', 'papel'] as grupo}
          <h2 class="grupo-temas">
            {textos.ui[
              grupo === 'oscuros'
                ? 'temasOscuros'
                : grupo === 'claros'
                  ? 'temasClaros'
                  : 'temasPapel'
            ]}
          </h2>
          <div class="temas">
            {#each temas.filter((t) => grupoDeTema(t) === grupo) as tema}
              <button
                class="tema"
                class:seleccionado={aparienciaActual.actual.tema === tema.id}
                aria-pressed={aparienciaActual.actual.tema === tema.id}
                disabled={acciones.ocupado}
                onclick={() =>
                  void acciones.hacer(() => aparienciaActual.cambiar('tema', tema.id), false)}
                style:--muestra-fondo={tema.colores.fondo}
                style:--muestra-superficie={tema.colores.superficieAlt}
                style:--muestra-acento={tema.colores.acento}
              >
                <span class="muestra" aria-hidden="true"
                  ><i class="muestra-lateral"></i><i class="muestra-superior"></i><i
                    class="muestra-tarjeta"
                  ></i><i class="muestra-acento"></i></span
                ><span class="nombre-tema"
                  >{textos.temas[tema.id as keyof typeof textos.temas]}</span
                >
              </button>
            {/each}
          </div>
        {/each}
      </section>
    {:else if seccion === 'avisos'}<section class="panel">
        <h2>{textos.ui.avisos}</h2>
        <EstadoCarga
          cargando={configuracion.cargando}
          fallo={configuracion.fallo}
          reintentar={() => void cargar()}
        />{#if borrador && !configuracion.fallo}<form
            onsubmit={(e) => {
              e.preventDefault();
              if (borrador) {
                const ajustes = {
                  ...$state.snapshot(borrador),
                  jornadaDesde: minutos(inicio),
                  jornadaHasta: minutos(fin)
                };
                void acciones.hacer(() => ipc.guardarAvisos(ajustes));
              }
            }}
          >
            <div class="campos">
              <label
                >{textos.ui.pausaCada}<input
                  type="number"
                  bind:value={borrador.pausaCadaMin}
                  min="0"
                  max="240"
                  step="1"
                  required
                /></label
              ><label
                >{textos.ui.vistaCada}<input
                  type="number"
                  bind:value={borrador.vistaCadaMin}
                  min="0"
                  max="120"
                  step="1"
                  required
                /></label
              ><label
                >{textos.ui.aguaCada}<input
                  type="number"
                  bind:value={borrador.aguaCadaMin}
                  min="0"
                  max="240"
                  step="1"
                  required
                /></label
              ><label
                >{textos.ui.jornadaDesde}<input type="time" bind:value={inicio} required /></label
              ><label>{textos.ui.jornadaHasta}<input type="time" bind:value={fin} required /></label
              >
            </div>
            <DiasSemana bind:valor={borrador.jornadaDias} /><label class="check"
              ><input type="checkbox" bind:checked={borrador.tomas} />{textos.ui.avisosTomas}</label
            ><label class="check"
              ><input type="checkbox" bind:checked={borrador.seguirEnBandeja} />{textos.ui
                .bandeja}</label
            >
            <div class="acciones">
              <button class="primario" disabled={acciones.ocupado}>{textos.comun.guardar}</button
              ><button
                type="button"
                disabled={acciones.ocupado}
                onclick={() => void acciones.hacer(() => ipc.avisosPosponer(30))}
                >{textos.ui.posponer}</button
              >
            </div>
          </form>{/if}<label class="check"
          ><input
            type="checkbox"
            checked={autoinicio}
            disabled={acciones.ocupado || (gestionada && !autoinicio)}
            onchange={(e) => {
              const activo = e.currentTarget.checked;
              e.currentTarget.checked = autoinicio;
              void acciones.hacer(async () => (autoinicio = await ipc.inicioConSistema(activo)));
            }}
          />{textos.ui.inicioSistema}</label
        >{#if gestionada}<p class="ayuda">{textos.ui.inicioLauncher}</p>{/if}
      </section>
    {:else if seccion === 'telefono'}{#key reintentoTelefono}{#await import('#lib/ui/BloqueTelefono.svelte')}<EstadoCarga
            cargando={true}
            fallo={null}
            reintentar={() => reintentoTelefono++}
          />{:then modulo}<modulo.default />{:catch}<EstadoCarga
            cargando={false}
            fallo="COMPONENT_UNAVAILABLE"
            reintentar={() => reintentoTelefono++}
          />{/await}{/key}
    {:else if seccion === 'copias'}{#key reintentoCopia}{#await import('#lib/ui/BloqueCopia.svelte')}<EstadoCarga
            cargando={true}
            fallo={null}
            reintentar={() => reintentoCopia++}
          />{:then modulo}<modulo.default />{:catch}<EstadoCarga
            cargando={false}
            fallo="COMPONENT_UNAVAILABLE"
            reintentar={() => reintentoCopia++}
          />{/await}{/key}
    {:else if seccion === 'red'}<section class="panel">
        <h2>{textos.ui.red}</h2>
        <p class="ayuda">{textos.ui.sinTelemetria}</p>
        <EstadoCarga
          cargando={red.cargando}
          fallo={red.fallo}
          reintentar={() => void red.cargar(ipc.politicaRed)}
        />{#if red.datos && !red.fallo}<label class="check"
            ><input
              type="checkbox"
              checked={red.datos.general}
              disabled={acciones.ocupado}
              onchange={(e) => {
                const valor = e.currentTarget.checked;
                e.currentTarget.checked = red.datos?.general ?? false;
                void acciones.hacer(() => politica(null, valor));
              }}
            />{textos.ui.permitirRed}</label
          ><label class="check"
            ><input
              type="checkbox"
              checked={red.datos.servicios.includes('openFoodFacts')}
              disabled={acciones.ocupado}
              onchange={(e) => {
                const valor = e.currentTarget.checked;
                e.currentTarget.checked = red.datos?.servicios.includes('openFoodFacts') ?? false;
                void acciones.hacer(() => politica('openFoodFacts', valor));
              }}
            />{textos.ui.off}</label
          >
          <p class="ayuda">{textos.ui.offConsentimiento}</p>
          <label class="check"
            ><input
              type="checkbox"
              checked={red.datos.servicios.includes('actualizaciones')}
              disabled={acciones.ocupado}
              onchange={(e) => {
                const valor = e.currentTarget.checked;
                e.currentTarget.checked = red.datos?.servicios.includes('actualizaciones') ?? false;
                void acciones.hacer(() => politica('actualizaciones', valor));
              }}
            />{textos.ui.actualizaciones}</label
          >
          <p class="ayuda">{textos.ui.actualizacionesConsentimiento}</p>{/if}
      </section>
    {:else if seccion === 'actualizaciones'}<section class="panel">
        <h2>{textos.ui.actualizaciones}</h2>
        {#if gestionada}<p class="ayuda">{textos.ui.actualizacionesLauncher}</p>{:else}
          <button
            type="button"
            disabled={acciones.ocupado ||
              !red.datos?.general ||
              !red.datos?.servicios.includes('actualizaciones')}
            onclick={() =>
              void acciones.hacer(async () => {
                actualizacion = await ipc.actualizacionBuscar();
                buscada = true;
              }, false)}>{textos.ui.buscarActualizacion}</button
          >{#if actualizacion}<div class="resultado">
              <strong>{actualizacion.version}</strong>
              <p>{actualizacion.notas ?? ''}</p>
              <button
                type="button"
                disabled={acciones.ocupado ||
                  !red.datos?.general ||
                  !red.datos?.servicios.includes('actualizaciones')}
                onclick={() => void acciones.hacer(ipc.actualizacionInstalar, false)}
                >{textos.ui.instalarActualizacion}</button
              >
            </div>{:else if buscada}<p class="ayuda">{textos.ui.sinActualizacion}</p>{/if}
        {/if}
      </section>
    {:else if seccion === 'acerca'}<section class="panel">
        <h2>{textos.ui.acerca}</h2>
        <p>{textos.app.nombre} · {version}</p>
        <p class="ayuda">{textos.ui.sinTelemetria}</p>
        <p class="ayuda">{textos.comun.estimacion}</p>
      </section>
    {/if}
  </div>
</div>

<style>
  .ajustes-distribucion {
    display: grid;
    grid-template-columns: 13rem minmax(0, 1fr);
    gap: var(--sc-espacio-5);
    align-items: start;
  }
  .apartados {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-1);
    position: sticky;
    top: 0;
    padding-top: var(--sc-espacio-5);
  }
  .apartados button {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    text-align: left;
    border: 0;
    background: transparent;
    font-weight: 450;
    padding: var(--sc-espacio-3) var(--sc-espacio-4);
    min-height: 2.8rem;
  }
  .apartados .activo {
    background: var(--m3c-primary);
    color: var(--m3c-on-primary);
    font-weight: 600;
  }
  .apartados :global(svg) {
    flex-shrink: 0;
  }
  .ajustes-contenido {
    min-width: 0;
    outline-offset: var(--sc-espacio-1);
  }
  h1 {
    margin: 0 0 var(--sc-espacio-5);
  }
  .apariencia {
    background: transparent;
    border: 1px solid var(--m3c-outline-variant);
  }
  .fila-ajuste {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sc-espacio-4);
    padding-bottom: var(--sc-espacio-5);
    border-bottom: 1px solid var(--m3c-outline-variant);
  }
  .fila-ajuste h2 {
    margin: 0;
    font-size: var(--sc-texto-md);
  }
  .idiomas {
    display: flex;
    padding: var(--sc-espacio-1);
    border-radius: var(--sc-radio-pleno);
    border: 1px solid var(--m3c-outline-variant);
  }
  .idiomas button {
    border: 0;
    background: transparent;
    min-height: 2.2rem;
    padding: var(--sc-espacio-2) var(--sc-espacio-3);
  }
  .idiomas .seleccionado {
    background: var(--m3c-primary);
    color: var(--m3c-on-primary);
  }
  .tipografia {
    margin: var(--sc-espacio-5) 0;
    align-items: end;
    grid-template-columns: 1fr 1fr;
  }
  .escala {
    display: flex;
    align-items: end;
    gap: var(--sc-espacio-2);
    margin: 0;
  }
  .escala label {
    min-width: 0;
    flex: 1;
  }
  .escala input {
    width: 100%;
  }
  .grupo-temas {
    font-size: var(--sc-texto-sm);
    margin: var(--sc-espacio-5) 0 var(--sc-espacio-3);
  }
  .temas {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: var(--sc-espacio-2);
  }
  .tema {
    background: transparent;
    border: 1px solid var(--m3c-outline-variant);
    border-radius: var(--sc-radio-md);
    padding: var(--sc-espacio-2);
    text-align: left;
    min-width: 0;
    font-weight: 450;
  }
  .tema.seleccionado {
    border-color: var(--m3c-primary);
    outline: 1px solid var(--m3c-primary);
  }
  .muestra {
    display: block;
    position: relative;
    background: var(--muestra-fondo);
    height: 3.5rem;
    border-radius: var(--sc-radio-sm);
    overflow: hidden;
  }
  .muestra i {
    position: absolute;
    display: block;
    border-radius: var(--sc-radio-sm);
    background: var(--muestra-superficie);
  }
  .muestra-lateral {
    inset: 0.2rem auto 0.2rem 0.2rem;
    width: 18%;
  }
  .muestra-superior {
    top: 0.2rem;
    left: 23%;
    right: 0.2rem;
    height: 0.5rem;
  }
  .muestra-tarjeta {
    left: 23%;
    right: 0.2rem;
    bottom: 0.2rem;
    height: 65%;
  }
  .muestra .muestra-acento {
    bottom: 0.45rem;
    right: 0.45rem;
    width: 0.8rem;
    height: 1.1rem;
    background: var(--muestra-acento);
  }
  .nombre-tema {
    display: block;
    margin-top: var(--sc-espacio-2);
    font-size: var(--sc-texto-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .ajustes-distribucion {
    grid-template-columns: 10rem minmax(0, 1fr);
    gap: var(--sc-espacio-3);
  }
  :global(.carcasa[data-ancho='compacto']) .ajustes-distribucion {
    grid-template-columns: 1fr;
  }
  :global(.carcasa[data-ancho='compacto']) .apartados {
    position: static;
    flex-direction: row;
    overflow: auto;
    padding-top: 0;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .fila-ajuste {
    flex-direction: column;
    align-items: flex-start;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .tipografia {
    grid-template-columns: 1fr;
  }
</style>
