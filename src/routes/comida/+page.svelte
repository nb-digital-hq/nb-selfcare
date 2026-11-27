<script lang="ts" module>
  /** La sección elegida sobrevive al remontaje que provoca un cambio de datos. */
  const memoria = { vista: 'hoy' };
</script>

<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { onDestroy } from 'svelte';
  import { busquedaGlobal } from '#lib/core/busqueda.svelte.ts';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  import Secciones from '#lib/ui/Secciones.svelte';
  import Icon from '#lib/ui/Icon.svelte';
  import Cajon from '#lib/ui/Cajon.svelte';
  import Metrica from '#lib/ui/Metrica.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import EstadoVacio from '#lib/ui/EstadoVacio.svelte';
  import BotonEliminar from '#lib/ui/BotonEliminar.svelte';
  const acciones = new Acciones();
  const datos = new Recurso<ipc.DiaComida>();
  const busqueda = new Recurso<ipc.Alimento[]>();
  const combos = new Recurso<ipc.Combo[]>();
  let dia = $state(diaLocal());
  let vista = $state(memoria.vista);
  $effect(() => {
    memoria.vista = vista;
  });
  const secciones = $derived([
    { id: 'hoy', titulo: textos.comun.hoy },
    { id: 'catalogo', titulo: textos.ui.buscarAlimento },
    { id: 'combinaciones', titulo: textos.ui.combinaciones },
    { id: 'recetas', titulo: textos.ui.recetas },
    { id: 'propio', titulo: textos.ui.alimentoPropio }
  ]);
  let consulta = $state('');
  $effect(() => {
    void busquedaGlobal.revision;
    consulta = busquedaGlobal.consulta;
    if (busquedaGlobal.revision > 0) vista = 'catalogo';
  });
  let comida = $state('lunch');
  let gramos = $state(100);
  let elegido = $state<ipc.Alimento | null>(null);
  let frase = $state('');
  let propuesta = $state<ipc.Propuesta[] | null>(null);
  let fraseRevisada = $state('');
  let codigo = $state('');
  let agua = $state(250);
  let nombreCombo = $state('');
  let escala = $state(1);
  let propio = $state<ipc.NuevoAlimento>({
    nombre: '',
    marca: null,
    porcionG: 100,
    kcal: 0,
    proteina: 0,
    carbohidratos: 0,
    grasa: 0,
    fibra: 0,
    sodio: 0
  });
  const cargar = () => datos.cargar(() => ipc.comidaDia(dia));
  $effect(() => {
    void cambios.version;
    void cargar();
    void combos.cargar(ipc.comidaCombos);
  });
  // Una nueva consulta cancela el permiso de la anterior para pintar resultados.
  $effect(() => {
    const q = consulta;
    const t = setTimeout(() => void busqueda.cargar(() => ipc.comidaBuscar(q, 30)), 180);
    return () => {
      clearTimeout(t);
      busqueda.cancelar();
    };
  });
  onDestroy(() => {
    datos.cancelar();
    busqueda.cancelar();
    combos.cancelar();
  });
  async function seleccionar(id: number) {
    await acciones.hacer(async () => {
      elegido = await ipc.comidaAlimento(id);
      gramos = elegido.porcionG;
    }, false);
  }
</script>

<div class="cabecera-vista">
  <div>
    <h1>{textos.nav.comida}</h1>
    <p class="subtitulo">{textos.ui.comidaSub}</p>
  </div>
  <button class="primario" onclick={() => (vista = 'catalogo')}
    ><Icon nombre="mas" tamano={17} />{textos.ui.registroRapido}</button
  >
</div>
<div class="campos filtros-comida">
  <label>{textos.ui.fecha}<input type="date" bind:value={dia} max={diaLocal()} /></label
  ><MenuSelector
    rotulo={textos.ui.momento}
    bind:valor={comida}
    opciones={Object.entries(textos.comidas).map(([valor, etiqueta]) => ({
      valor: valor as typeof comida,
      etiqueta
    }))}
  />
</div>
<Secciones clave="comida" opciones={secciones} bind:actual={vista} etiqueta={textos.nav.comida} />
<div id="comida-contenido" role="tabpanel" aria-labelledby={`comida-${vista}`}>
  <EstadoCarga cargando={datos.cargando} fallo={datos.fallo} reintentar={() => void cargar()} />
  {#if datos.datos && !datos.fallo}
    <div class="metricas">
      <Metrica
        titulo={textos.ui.energia}
        valor={datos.datos.totales.kcal}
        meta={datos.datos.metas.kcal}
        unidad="kcal"
      /><Metrica
        titulo={textos.ui.proteina}
        valor={datos.datos.totales.proteina}
        meta={datos.datos.metas.proteinaG}
        unidad="g"
      /><Metrica
        titulo={textos.ui.carbohidratos}
        valor={datos.datos.totales.carbohidratos}
        meta={datos.datos.metas.carbohidratosG}
        unidad="g"
      /><Metrica
        titulo={textos.ui.grasa}
        valor={datos.datos.totales.grasa}
        meta={datos.datos.metas.grasaG}
        unidad="g"
      />
    </div>
  {/if}
  {#if vista === 'hoy'}{#if datos.datos && !datos.fallo}
      <section class="panel">
        <h2>{textos.ui.registros}</h2>
        {#each datos.datos.registros as registro (registro.id)}<div class="fila">
            <div>
              <strong>{registro.nombre}</strong>
              <p class="detalle">
                {textos.comidas[registro.comida as keyof typeof textos.comidas] ??
                  textos.ui.momento} · {registro.hora}
                · {numero(registro.gramos)}
                {textos.unidades.g} · {numero(registro.kcal)}
                {textos.unidades.kcal}
              </p>
            </div>
            <BotonEliminar eliminar={() => ipc.comidaBorrar([registro.id])} />
          </div>{:else}<EstadoVacio titulo={textos.comun.vacio} />{/each}<button
          type="button"
          disabled={acciones.ocupado || !datos.datos.hayAyer || dia !== diaLocal()}
          onclick={() => void acciones.hacer(ipc.comidaRepetirAyer)}>{textos.ui.repetirAyer}</button
        >
      </section>
      <div class="dos-columnas">
        <section class="panel">
          <h2>{textos.ui.agua}</h2>
          <Metrica
            titulo={textos.ui.agua}
            valor={datos.datos.aguaMl}
            meta={datos.datos.aguaMetaMl}
            unidad="ml"
          />
          <form
            onsubmit={(e) => {
              e.preventDefault();
              void acciones.hacer(() => ipc.comidaBeber(agua));
            }}
          >
            <label
              >{textos.ui.volumen}<input
                type="number"
                bind:value={agua}
                min="10"
                max="3000"
                step="10"
                required
              /></label
            ><button disabled={acciones.ocupado || dia !== diaLocal()}
              >{textos.comun.registrar}</button
            >
          </form>
          {#each datos.datos.agua as vaso (vaso.id)}<div class="fila">
              <span>{vaso.ml} {textos.unidades.ml} · {vaso.hora}</span><BotonEliminar
                eliminar={() => ipc.comidaBorrarVaso(vaso.id)}
              />
            </div>{/each}
        </section>
        <section class="panel">
          <h2>{textos.ui.nutrientes}</h2>
          {#each [['fibra', 'fibraG', 'g'], ['sodio', 'sodioMg', 'mg'], ['potasio', 'potasioMg', 'mg'], ['magnesio', 'magnesioMg', 'mg']] as [k, m, u]}<div
              class="fila"
            >
              <span>{textos.ui[k as 'fibra' | 'sodio' | 'potasio' | 'magnesio']}</span><strong
                >{numero(datos.datos.totales[k as 'fibra' | 'sodio' | 'potasio' | 'magnesio'])} / {numero(
                  datos.datos.metas[m as 'fibraG' | 'sodioMg' | 'potasioMg' | 'magnesioMg']
                )}
                {u}</strong
              >
            </div>{/each}
          <p class="ayuda">{textos.comun.estimacion}</p>
        </section>
      </div>
    {/if}
  {:else if vista === 'catalogo'}<div class="dos-columnas catalogo">
      <section class="panel">
        <h2>{textos.ui.buscarAlimento}</h2>
        <label
          >{textos.ui.buscar}<input type="search" bind:value={consulta} maxlength="200" /></label
        ><EstadoCarga
          cargando={busqueda.cargando}
          fallo={busqueda.fallo}
          reintentar={() => void busqueda.cargar(() => ipc.comidaBuscar(consulta, 30))}
        />{#if busqueda.datos && !busqueda.fallo}{#each busqueda.datos as a (a.id)}<div
              class="fila"
            >
              <button type="button" class="alimento" onclick={() => void seleccionar(a.id)}
                ><strong>{a.nombre}</strong><small
                  >{numero(a.kcal)} {textos.unidades.kcal} / {textos.ui.por100g}</small
                ></button
              ><button
                type="button"
                disabled={acciones.ocupado}
                aria-label={textos.ui.favorito}
                aria-pressed={a.favorito}
                onclick={() =>
                  void acciones.hacer(async () => {
                    await ipc.comidaFavorito(a.id, !a.favorito);
                    await busqueda.cargar(() => ipc.comidaBuscar(consulta, 30));
                  }, false)}><Icon nombre="favorito" /></button
              >
            </div>{:else}<EstadoVacio titulo={textos.comun.sinResultados} />{/each}{/if}
      </section>
      <div>
        <form
          class="panel"
          onsubmit={(e) => {
            e.preventDefault();
            void acciones.hacer(async () => {
              const enviada = frase;
              const leida = await ipc.comidaProponer(enviada);
              if (frase !== enviada) return;
              propuesta = leida;
              fraseRevisada = enviada;
            }, false);
          }}
        >
          <h2>{textos.ui.frase}</h2>
          <label
            >{textos.ui.fraseEjemplo}<textarea
              bind:value={frase}
              maxlength="1000"
              required
              oninput={() => (propuesta = null)}></textarea></label
          ><button disabled={acciones.ocupado}>{textos.ui.revisarPropuesta}</button
          >{#if propuesta?.length}<div class="resultado">
              {#each propuesta as p}<p>
                  <strong>{p.alimento?.nombre ?? p.leido.termino}</strong> · {numero(
                    p.leido.gramos
                  )}
                  {textos.unidades.g} · {numero(p.kcal)}
                  {textos.unidades.kcal}
                </p>
                {#if !p.alimento}<p class="ayuda">{textos.ui.sinCatalogo}</p>{/if}{/each}<button
                type="button"
                class="primario"
                disabled={acciones.ocupado || frase !== fraseRevisada || dia !== diaLocal()}
                onclick={() =>
                  void acciones.hacer(async () => {
                    await ipc.comidaRegistrarFrase(fraseRevisada, comida);
                    propuesta = null;
                    frase = '';
                  })}>{textos.ui.confirmarComida}</button
              >
            </div>{/if}
        </form>
        <form
          class="panel"
          onsubmit={(e) => {
            e.preventDefault();
            void acciones.hacer(async () => {
              elegido = await ipc.comidaCodigo(codigo);
              gramos = elegido.porcionG;
            }, false);
          }}
        >
          <h2>{textos.ui.codigo}</h2>
          <label
            >{textos.ui.codigo}<input
              bind:value={codigo}
              inputmode="numeric"
              pattern={'[0-9]{8,14}'}
              required
            /></label
          >
          <p class="ayuda">{textos.ui.codigoAyuda}</p>
          <button disabled={acciones.ocupado}>{textos.ui.buscar}</button>
        </form>
      </div>
    </div>
  {:else if vista === 'combinaciones'}<section class="panel">
      <h2>{textos.ui.combinaciones}</h2>
      <form
        class="campos"
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(async () => {
            await ipc.comidaGuardarCombo(nombreCombo, dia, comida);
            nombreCombo = '';
          });
        }}
      >
        <label
          >{textos.ui.nombreCombo}<input bind:value={nombreCombo} required maxlength="80" /></label
        ><button disabled={acciones.ocupado}>{textos.ui.guardarCombo}</button>
      </form>
      <label
        >{textos.ui.escala}<input
          type="number"
          bind:value={escala}
          min="0.1"
          max="10"
          step="0.1"
        /></label
      ><EstadoCarga
        cargando={combos.cargando}
        fallo={combos.fallo}
        reintentar={() => void combos.cargar(ipc.comidaCombos)}
      />{#each combos.datos ?? [] as combo (combo.id)}<div class="fila">
          <div>
            <strong>{combo.nombre}</strong>
            <p class="detalle">{numero(combo.kcal)} {textos.unidades.kcal}</p>
          </div>
          <div class="acciones">
            <button
              type="button"
              disabled={acciones.ocupado || dia !== diaLocal()}
              onclick={() =>
                void acciones.hacer(() => ipc.comidaRegistrarCombo(combo.id, escala, comida))}
              >{textos.comun.registrar}</button
            ><BotonEliminar eliminar={() => ipc.comidaBorrarCombo(combo.id)} />
          </div>
        </div>{/each}
    </section>
  {:else if vista === 'recetas'}{#await import('#lib/ui/BloqueRecetas.svelte') then modulo}<modulo.default
        {dia}
        {comida}
      />{/await}
  {:else if vista === 'propio'}<section class="panel">
      <h2>{textos.ui.alimentoPropio}</h2>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(async () => {
            elegido = await ipc.comidaCrear($state.snapshot(propio));
            gramos = elegido.porcionG;
          });
        }}
      >
        <p class="ayuda">{textos.ui.por100}</p>
        <div class="campos">
          <label
            >{textos.ui.nombre}<input bind:value={propio.nombre} required maxlength="120" /></label
          ><label
            >{textos.ui.porcion}<input
              type="number"
              bind:value={propio.porcionG}
              min="1"
              max="5000"
              required
            /></label
          >{#each ['kcal', 'proteina', 'carbohidratos', 'grasa', 'fibra', 'sodio'] as k}<label
              >{textos.ui[
                k as 'kcal' | 'proteina' | 'carbohidratos' | 'grasa' | 'fibra' | 'sodio'
              ]}<input
                type="number"
                min="0"
                max="100000"
                step="0.1"
                required
                bind:value={
                  propio[k as 'kcal' | 'proteina' | 'carbohidratos' | 'grasa' | 'fibra' | 'sodio']
                }
              /></label
            >{/each}
        </div>
        <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
      </form>
    </section>
  {/if}
</div>
{#if elegido}<Cajon titulo={elegido.nombre} icono="comida" oncerrar={() => (elegido = null)}
    ><form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(async () => {
          if (elegido) await ipc.comidaRegistrar(elegido.id, gramos, comida);
          elegido = null;
        });
      }}
    >
      <h2>{elegido.nombre}</h2>
      <p class="ayuda">
        {textos.ui.por100} · {numero(elegido.kcal)}
        {textos.unidades.kcal} · {textos.ui.proteina}: {numero(elegido.proteina, 1)}
        {textos.unidades.g} · {textos.ui.nova}
        {elegido.nova ?? '—'}
      </p>
      <div class="campos">
        <label
          >{textos.ui.gramos}<input
            type="number"
            bind:value={gramos}
            min="1"
            max="5000"
            step="0.1"
            required
          /></label
        >
      </div>
      <div class="acciones">
        <button class="primario" disabled={acciones.ocupado || dia !== diaLocal()}
          >{textos.comun.registrar}</button
        ><button type="button" onclick={() => (elegido = null)}>{textos.comun.cancelar}</button>
      </div>
    </form></Cajon
  >{/if}

<style>
  .filtros-comida {
    grid-template-columns: minmax(9rem, 12rem) minmax(9rem, 12rem);
    margin-bottom: var(--sc-espacio-4);
  }
  .catalogo {
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
  }
  :global(.carcasa:not([data-ancho='amplio'])) .catalogo {
    grid-template-columns: 1fr;
  }

  .alimento {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-1);
    align-items: flex-start;
    text-align: left;
    background: none;
    border: 0;
    padding: 0;
  }
  .alimento small {
    color: var(--m3c-on-surface-variant);
  }
</style>
