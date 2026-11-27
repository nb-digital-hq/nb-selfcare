<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { perfilBienvenida, type Bienvenida } from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import BloqueCopia from './BloqueCopia.svelte';
  import Logo from './Logo.svelte';
  const acciones = new Acciones();
  let b = $state<Bienvenida>({
    nombre: '',
    nacimiento: '',
    sexo: 'hombre',
    alturaCm: 170,
    pesoActualKg: 70,
    pesoMetaKg: 70,
    ritmoSemanalKg: 0,
    aguaMetaMl: 2000,
    pasosMeta: 6000,
    suenoMetaH: 8,
    antecedentesTca: false,
    tomaHipoglucemiantes: false,
    aparatos: []
  });
</script>

<div class="bienvenida">
  <div class="presentacion">
    <Logo />
    <p class="ceja">{textos.app.nombre}</p>
    <h1>{textos.ui.bienvenida}</h1>
    <p class="subtitulo">{textos.ui.bienvenidaSub}</p>
  </div>
  <form
    class="panel"
    onsubmit={(e) => {
      e.preventDefault();
      void acciones.hacer(() => perfilBienvenida($state.snapshot(b)));
    }}
  >
    <div class="campos">
      <label
        >{textos.ui.nombre}<input
          bind:value={b.nombre}
          required
          maxlength="60"
          autocomplete="nickname"
        /></label
      >
      <label>{textos.ui.nacimiento}<input type="date" bind:value={b.nacimiento} required /></label>
      <MenuSelector
        rotulo={textos.ui.sexo}
        bind:valor={b.sexo}
        opciones={[
          { valor: 'hombre' as typeof b.sexo, etiqueta: textos.ui.hombre },
          { valor: 'mujer' as typeof b.sexo, etiqueta: textos.ui.mujer }
        ]}
      />
      <label
        >{textos.ui.altura}<input
          type="number"
          bind:value={b.alturaCm}
          min="100"
          max="250"
          step="0.1"
          required
        /></label
      >
      <label
        >{textos.ui.peso}<input
          type="number"
          bind:value={b.pesoActualKg}
          min="25"
          max="350"
          step="0.1"
          required
        /></label
      >
      <label
        >{textos.ui.pesoMeta}<input
          type="number"
          bind:value={b.pesoMetaKg}
          min="25"
          max="350"
          step="0.1"
          required
        /></label
      >
      <label
        >{textos.ui.aguaMeta}<input
          type="number"
          bind:value={b.aguaMetaMl}
          min="500"
          max="6000"
          step="100"
          required
        /></label
      >
      <label
        >{textos.ui.suenoMeta}<input
          type="number"
          bind:value={b.suenoMetaH}
          min="3"
          max="14"
          step="0.5"
          required
        /></label
      >
    </div>
    <label class="check"
      ><input type="checkbox" bind:checked={b.antecedentesTca} />{textos.ui.tca}</label
    >
    <label class="check"
      ><input type="checkbox" bind:checked={b.tomaHipoglucemiantes} />{textos.ui
        .hipoglucemiantes}</label
    >
    <p class="ayuda">{textos.comun.estimacion}</p>
    <button class="primario" disabled={acciones.ocupado}>{textos.ui.crearPerfil}</button>
  </form>
  <p class="ayuda vincular">{textos.ui.vincularAlEmpezar}</p>
  {#await import('./BloqueTelefono.svelte') then modulo}<modulo.default />{/await}
  <BloqueCopia />
</div>

<style>
  .vincular {
    margin: var(--sc-espacio-6) 0 var(--sc-espacio-3);
  }
  .bienvenida {
    max-width: 55rem;
    margin: var(--sc-espacio-5) auto;
  }
  .presentacion {
    max-width: 40rem;
    margin-bottom: var(--sc-espacio-6);
    --logo-tamano: 4rem;
  }
  .ceja {
    color: var(--m3c-primary);
    font-weight: 650;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    font-size: var(--sc-texto-xs);
  }
</style>
