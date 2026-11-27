<script lang="ts" module>
  /** La sección elegida sobrevive al remontaje que provoca un cambio de datos. */
  const memoria = { vista: 'semana' };
</script>

<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { resumen } from '#lib/core/resumen.svelte.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  import Secciones from '#lib/ui/Secciones.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  const modulos = [
    ['treatments', 'tratamientos'],
    ['pelvic_floor', 'sueloPelvico'],
    ['active_breaks', 'pausas'],
    ['symptoms', 'sintomas'],
    ['checkups', 'chequeos'],
    ['habits', 'habitos'],
    ['substances', 'consumo'],
    ['quit', 'libre']
  ] as const;
  const acciones = new Acciones();
  const ciclo = new Recurso<ipc.Ciclo | null>();
  const inicial =
    typeof location !== 'undefined' ? new URLSearchParams(location.search).get('seccion') : null;
  let vista = $state(inicial ?? memoria.vista);
  $effect(() => {
    memoria.vista = vista;
  });
  const secciones = $derived([
    { id: 'semana', titulo: textos.ui.tuSemana },
    { id: 'tendencias', titulo: textos.ui.tendencias },
    { id: 'historial', titulo: textos.ui.historial },
    { id: 'objetivos', titulo: textos.ui.datosObjetivos },
    { id: 'salud', titulo: textos.ui.saludDispositivos },
    { id: 'macros', titulo: textos.ui.macrosManuales },
    ...(resumen.datos?.perfil?.cicloActivo ? [{ id: 'ciclo', titulo: textos.ui.ciclo }] : []),
    { id: 'plan', titulo: textos.ui.planSemanal }
  ]);
  $effect(() => {
    if (vista === 'ciclo' && !resumen.datos?.perfil?.cicloActivo) vista = 'salud';
  });
  let cambio = $state<ipc.CambioPerfil>({});
  let inicio = $state(diaLocal());
  let largo = $state(28);
  let kcal = $state(2000);
  let proteina = $state(100);
  let carbohidratos = $state(250);
  let grasa = $state(65);
  onMount(() => {
    const p = resumen.datos?.perfil;
    if (p)
      cambio = {
        modulos: [...p.modulos],
        aparatos: [...p.aparatos],
        nombre: p.nombre,
        alturaCm: p.alturaCm,
        pesoMetaKg: p.pesoMetaKg,
        ritmoSemanalKg: p.ritmoSemanalKg,
        aguaMetaMl: p.aguaMetaMl,
        pasosMeta: p.pasosMeta,
        suenoMetaH: p.suenoMetaH,
        arquetipo: p.arquetipo,
        fase: p.fase,
        cicloActivo: p.cicloActivo,
        ayunoActivo: p.ayunoActivo,
        antecedentesTca: p.antecedentesTca,
        tomaHipoglucemiantes: p.tomaHipoglucemiantes,
        avisosDiscretos: p.avisosDiscretos
      };
  });
  $effect(() => {
    void cambios.version;
    void ciclo.cargar(ipc.ciclo);
  });
  onDestroy(() => ciclo.cancelar());
</script>

<h1>{textos.nav.perfil}</h1>
<p class="subtitulo">{textos.ui.perfilSub}</p>
<Secciones clave="perfil" opciones={secciones} bind:actual={vista} etiqueta={textos.nav.perfil} />
<div id="perfil-contenido" role="tabpanel" aria-labelledby={`perfil-${vista}`}>
  {#if vista === 'semana' || vista === 'tendencias' || vista === 'historial'}{#await import('#lib/ui/evolucion/Evolucion.svelte') then modulo}<modulo.default
        {vista}
      />{/await}
  {:else if vista === 'objetivos' || vista === 'salud'}<form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => ipc.perfilCambiar($state.snapshot(cambio)));
      }}
    >
      {#if vista === 'objetivos'}
        <div class="campos">
          <label
            >{textos.ui.nombre}<input bind:value={cambio.nombre} maxlength="60" required /></label
          ><label
            >{textos.ui.altura}<input
              type="number"
              bind:value={cambio.alturaCm}
              min="100"
              max="250"
              step="0.1"
              required
            /></label
          ><label
            >{textos.ui.pesoMeta}<input
              type="number"
              bind:value={cambio.pesoMetaKg}
              min="25"
              max="350"
              step="0.1"
              required
            /></label
          ><label
            >{textos.ui.ritmo}<input
              type="number"
              bind:value={cambio.ritmoSemanalKg}
              min="-1.5"
              max="1"
              step="0.1"
              required
            /></label
          ><label
            >{textos.ui.aguaMeta}<input
              type="number"
              bind:value={cambio.aguaMetaMl}
              min="500"
              max="6000"
              step="100"
              required
            /></label
          ><label
            >{textos.ui.pasosMeta}<input
              type="number"
              bind:value={cambio.pasosMeta}
              min="0"
              max="60000"
              required
            /></label
          ><label
            >{textos.ui.suenoMeta}<input
              type="number"
              bind:value={cambio.suenoMetaH}
              min="3"
              max="14"
              step="0.5"
              required
            /></label
          ><MenuSelector
            rotulo={textos.ui.fase}
            valor={cambio.fase ?? 'deficit'}
            onchange={(v) => (cambio.fase = v)}
            opciones={Object.entries(textos.fases).map(([valor, etiqueta]) => ({
              valor: valor as NonNullable<typeof cambio.fase>,
              etiqueta
            }))}
          /><MenuSelector
            rotulo={textos.ui.arquetipo}
            valor={cambio.arquetipo ?? 'flexible'}
            onchange={(v) => (cambio.arquetipo = v)}
            opciones={Object.entries(textos.estilos).map(([valor, etiqueta]) => ({
              valor: valor as NonNullable<typeof cambio.arquetipo>,
              etiqueta
            }))}
          />
        </div>
      {:else}
        {#each [['cicloActivo', 'ciclo'], ['ayunoActivo', 'habilitarAyuno'], ['antecedentesTca', 'tca'], ['tomaHipoglucemiantes', 'hipoglucemiantes'], ['avisosDiscretos', 'discretos']] as [campo, etiqueta]}<label
            class="check"
            ><input
              type="checkbox"
              bind:checked={
                cambio[
                  campo as
                    | 'cicloActivo'
                    | 'ayunoActivo'
                    | 'antecedentesTca'
                    | 'tomaHipoglucemiantes'
                    | 'avisosDiscretos'
                ]
              }
            />{textos.ui[
              etiqueta as 'ciclo' | 'habilitarAyuno' | 'tca' | 'hipoglucemiantes' | 'discretos'
            ]}</label
          >{/each}
        <fieldset class="panel">
          <legend>{textos.ui.modulos}</legend>
          {#each modulos as [clave, rotulo]}
            <label class="check"
              ><input
                type="checkbox"
                checked={cambio.modulos?.includes(clave) ?? false}
                onchange={(e) => {
                  const resto = (cambio.modulos ?? []).filter((k) => k !== clave);
                  cambio.modulos = e.currentTarget.checked ? [...resto, clave] : resto;
                }}
              />{textos.ui[
                rotulo as
                  | 'tratamientos'
                  | 'sueloPelvico'
                  | 'pausas'
                  | 'sintomas'
                  | 'chequeos'
                  | 'habitos'
                  | 'consumo'
                  | 'libre'
              ]}</label
            >
          {/each}
        </fieldset>
        <fieldset class="panel">
          <legend>{textos.ui.aparatos}</legend
          >{#each Object.entries(textos.vitales) as [clave, rotulo]}<label class="check"
              ><input
                type="checkbox"
                checked={cambio.aparatos?.includes(clave) ?? false}
                onchange={(e) => {
                  const resto = (cambio.aparatos ?? []).filter((k) => k !== clave);
                  cambio.aparatos = e.currentTarget.checked ? [...resto, clave] : resto;
                }}
              />{rotulo}</label
            >{/each}
        </fieldset>
      {/if}
      <p class="ayuda">{textos.comun.estimacion}</p>
      <button class="primario" disabled={acciones.ocupado}>{textos.comun.guardar}</button>
    </form>{:else if vista === 'macros'}<section class="panel">
      <h2>{textos.ui.macrosManuales}</h2>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(() =>
            ipc.perfilCambiar({
              macrosAMano: {
                kcal,
                proteinaG: proteina,
                carbohidratosG: carbohidratos,
                grasaG: grasa
              }
            })
          );
        }}
      >
        <div class="campos">
          <label
            >{textos.ui.kcal}<input
              type="number"
              bind:value={kcal}
              min="500"
              max="10000"
              required
            /></label
          ><label
            >{textos.ui.proteina}<input
              type="number"
              bind:value={proteina}
              min="0"
              max="1000"
              required
            /></label
          ><label
            >{textos.ui.carbohidratos}<input
              type="number"
              bind:value={carbohidratos}
              min="0"
              max="2000"
              required
            /></label
          ><label
            >{textos.ui.grasa}<input
              type="number"
              bind:value={grasa}
              min="0"
              max="1000"
              required
            /></label
          >
        </div>
        <div class="acciones">
          <button disabled={acciones.ocupado}>{textos.comun.guardar}</button><button
            type="button"
            disabled={acciones.ocupado}
            onclick={() => void acciones.hacer(() => ipc.perfilCambiar({ macrosAMano: null }))}
            >{textos.ui.macrosAutomaticos}</button
          >
        </div>
      </form>
    </section>{:else if vista === 'ciclo'}{#if resumen.datos?.perfil?.cicloActivo}<section
        class="panel"
      >
        <h2>{textos.ui.ciclo}</h2>
        <p class="ayuda">{textos.ui.cicloAyuda}</p>
        <EstadoCarga
          cargando={ciclo.cargando}
          fallo={ciclo.fallo}
          reintentar={() => void ciclo.cargar(ipc.ciclo)}
        />{#if ciclo.datos}<p>{ciclo.datos.inicio} / {ciclo.datos.proximo}</p>{/if}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void acciones.hacer(() => ipc.cicloEmpezar(inicio, largo));
          }}
        >
          <div class="campos">
            <label
              >{textos.ui.inicioCiclo}<input
                type="date"
                bind:value={inicio}
                max={diaLocal()}
                required
              /></label
            ><label
              >{textos.ui.duracionCiclo}<input
                type="number"
                bind:value={largo}
                min="15"
                max="60"
                required
              /></label
            >
          </div>
          <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
        </form>
      </section>{/if}
  {:else if vista === 'plan'}<section class="panel informe">
      <h2>{textos.ui.planSemanal}</h2>
      <p class="ayuda">{textos.comun.estimacion}</p>
      <div class="tabla-contenedor">
        <table class="tabla">
          <thead
            ><tr
              ><th>{textos.ui.fecha}</th><th>{textos.ui.kcal}</th><th>{textos.ui.proteina}</th><th
                >{textos.ui.agua}</th
              ></tr
            ></thead
          ><tbody
            >{#each Object.values(textos.dias) as dia}<tr
                ><td>{dia}</td><td>{numero(resumen.datos?.metas?.kcal)}</td><td
                  >{numero(resumen.datos?.metas?.proteinaG)} {textos.unidades.g}</td
                ><td>{numero(resumen.datos?.perfil?.aguaMetaMl)} {textos.unidades.ml}</td></tr
              >{/each}</tbody
          >
        </table>
      </div>
      <p class="ayuda">{textos.ui.informePrivado}</p>
      <button type="button" onclick={() => window.print()}>{textos.ui.informe}</button>
    </section>
  {/if}
</div>
