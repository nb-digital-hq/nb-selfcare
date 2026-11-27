<script lang="ts">
  import { onMount } from 'svelte';
  import { textos, idioma } from '#lib/i18n/index.ts';
  import { resumen } from '#lib/core/resumen.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import {
    diarioEntradas,
    diarioEscribir,
    diarioBorrar,
    diarioSugerir,
    nocheAnotar,
    cafeinaTomar,
    comidaBeber,
    type Entrada
  } from '#lib/core/ipc/index.ts';
  import Metrica from '#lib/ui/Metrica.svelte';
  import Anillo from '#lib/ui/Anillo.svelte';
  import Cajon from '#lib/ui/Cajon.svelte';
  import Icon from '#lib/ui/Icon.svelte';
  import EstadoCarga from '#lib/ui/EstadoCarga.svelte';
  import EstadoVacio from '#lib/ui/EstadoVacio.svelte';
  import BotonEliminar from '#lib/ui/BotonEliminar.svelte';
  import { minutos, numero } from '#lib/core/dia.ts';
  const acciones = new Acciones();
  const diario = new Recurso<Entrada[]>();
  let texto = $state('');
  let animo = $state(3);
  let energia = $state(3);
  let estres = $state(3);
  let horas = $state(8);
  let despertar = $state('07:00');
  let pulso = $state<number | undefined>();
  let pasos = $state<number | undefined>();
  let fuente = $state('');
  let mg = $state(80);
  let sugerida = $state(false);
  let registro = $state<'diario' | 'descanso' | 'cafeina' | null>(null);
  const datos = $derived(resumen.datos);
  const saludo = $derived(
    (datos?.hora ?? 12) < 12
      ? textos.ui.saludoManana
      : (datos?.hora ?? 12) < 19
        ? textos.ui.saludoTarde
        : textos.ui.saludoNoche
  );
  const fecha = $derived(
    datos
      ? new Intl.DateTimeFormat(idioma(), {
          weekday: 'long',
          day: 'numeric',
          month: 'long'
        })
          .format(new Date(`${datos.dia}T12:00:00`))
          .replace(/^\p{L}/u, (l) => l.toLocaleUpperCase(idioma()))
      : ''
  );
  const cargar = () => diario.cargar(() => diarioEntradas(30));
  $effect(() => {
    void cambios.version;
    void cargar();
  });
  onMount(() => {
    if (resumen.datos?.noche) {
      horas = resumen.datos.noche.minutos / 60;
      pulso = resumen.datos.noche.pulso ?? undefined;
      pasos = resumen.datos.noche.pasos ?? undefined;
    }
    return () => diario.cancelar();
  });
</script>

{#if datos?.perfil}
  <div class="cabecera-vista">
    <div>
      <p class="fecha">{fecha}</p>
      <h1>{saludo}, {datos.perfil.nombre}.</h1>
      <p class="subtitulo">{textos.ui.hoySub}</p>
    </div>
    <a class="perfil" href="/perfil" aria-label={textos.nav.perfil} title={textos.nav.perfil}
      >{datos.perfil.nombre.slice(0, 1).toUpperCase()}</a
    >
  </div>
  <div class="resumen-principal">
    <section class="panel protagonista">
      <div class="rotulo-dato"><Icon nombre="cuidado" tamano={18} />{textos.ui.disposicion}</div>
      {#if datos.disposicion}
        <div class="disposicion">
          <Anillo valor={datos.disposicion.total} etiqueta={textos.ui.disposicion} />
          <div>
            <h2>
              {textos.ui[datos.disposicion.categoria as 'optimal' | 'favorable' | 'recoveryNeeded']}
            </h2>
            <p class="ayuda">{textos.ui.disposicionSub}</p>
            <a class="enlace-discreto" href="/cuidado"
              >{textos.ui.verCuidado}<Icon nombre="derecha" tamano={16} /></a
            >
          </div>
        </div>
        <div class="componentes-disposicion">
          {#each [[textos.ui.aporteSueno, datos.disposicion.sueno, 40], [textos.ui.aportePulso, datos.disposicion.pulso, 35], [textos.ui.aporteCarga, datos.disposicion.carga, 25]] as parte}<div
            >
              <span>{parte[0]}</span><strong>{parte[1]}<small>/{parte[2]}</small></strong><progress
                max={Number(parte[2])}
                value={Number(parte[1])}
                aria-label={String(parte[0])}
              ></progress>
            </div>{/each}
        </div>
      {:else}<div class="sin-disposicion">
          <Icon nombre="cuidado" tamano={42} />
          <h2>{textos.ui.tuDia}</h2>
          <p class="ayuda">{textos.ui.sinDisposicion}</p>
          <button class="primario" onclick={() => (registro = 'descanso')}
            >{textos.ui.registrarDescanso}</button
          >
        </div>{/if}
    </section>
    <section class="panel balance">
      <div class="rotulo-dato"><Icon nombre="comida" tamano={18} />{textos.ui.energia}</div>
      <div class="balance-datos">
        <Anillo
          valor={datos.totales.kcal}
          maximo={datos.metas?.kcal ?? 1}
          etiqueta={textos.ui.energia}
          unidad={textos.unidades.kcal}
        />
        <div>
          <p class="ayuda">{textos.ui.objetivo}</p>
          <strong>{numero(datos.metas?.kcal ?? 0)} <small>{textos.unidades.kcal}</small></strong>
          <p class="ayuda">
            {textos.ui.restoDia}: {numero(
              Math.max(0, (datos.metas?.kcal ?? 0) - datos.totales.kcal)
            )}
            {textos.unidades.kcal}
          </p>
        </div>
      </div>
      <a class="enlace-accion" href="/comida"
        ><Icon nombre="mas" tamano={18} />{textos.ui.registroRapido}</a
      >
    </section>
  </div>
  {#await import('#lib/ui/evolucion/TarjetasHoy.svelte') then modulo}<modulo.default />{/await}
  <div class="metricas secundarias">
    <Metrica
      titulo={textos.ui.proteina}
      valor={datos.totales.proteina}
      meta={datos.metas?.proteinaG}
      unidad={textos.unidades.g}
    />
    <Metrica
      titulo={textos.ui.agua}
      valor={datos.aguaMl}
      meta={datos.perfil.aguaMetaMl}
      unidad={textos.unidades.ml}
    />
    <Metrica titulo={textos.ui.habitos} valor={datos.habitosHechos} meta={datos.habitosDelDia} />
  </div>
  <section class="cuidados-rapidos">
    <h2>{textos.ui.accionesHoy}</h2>
    <div class="atajos">
      <button
        disabled={acciones.ocupado}
        onclick={() => void acciones.hacer(() => comidaBeber(250))}
        ><Icon nombre="mas" tamano={20} /><span>{textos.ui.agua} +250 {textos.unidades.ml}</span
        ></button
      >
      <button onclick={() => (registro = 'diario')}
        ><Icon nombre="editar" tamano={20} /><span>{textos.ui.escribirDiario}</span></button
      >
      <button onclick={() => (registro = 'descanso')}
        ><Icon nombre="reloj" tamano={20} /><span>{textos.ui.registrarDescanso}</span></button
      >
      <button onclick={() => (registro = 'cafeina')}
        ><Icon nombre="mas" tamano={20} /><span>{textos.ui.cafeina}</span></button
      >
    </div>
  </section>
  <a class="resumen-ayuno" href="/ayuno"
    ><div class="rotulo-dato">
      <Icon nombre="reloj" tamano={23} />
      <div>
        <strong>{textos.nav.ayuno}</strong>
        <p>
          {datos.horasDeAyuno === null
            ? textos.ui.sinAyuno
            : `${numero(datos.horasDeAyuno, 1)} ${textos.unidades.h}`}
        </p>
      </div>
    </div>
    <Icon nombre="derecha" tamano={20} /></a
  >
  {#if registro === 'diario'}<Cajon
      titulo={textos.ui.diario}
      icono="editar"
      oncerrar={() => (registro = null)}
      ><form
        class="registro-formulario"
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(async () => {
            await diarioEscribir({ texto, animo, energia, estres, etiquetas: [] });
            texto = '';
            sugerida = false;
            registro = null;
          });
        }}
      >
        <h2>{textos.ui.diario}</h2>
        <p class="ayuda">{textos.ui.diarioSub}</p>
        <label
          >{textos.ui.nota}<textarea bind:value={texto} maxlength="5000" required></textarea></label
        >
        <div class="campos">
          {#each ['animo', 'energia', 'estres'] as k}<label
              >{k === 'animo'
                ? textos.ui.animo
                : k === 'energia'
                  ? textos.ui.energiaDiario
                  : textos.ui.estres}<input
                type="number"
                min="1"
                max="5"
                required
                value={k === 'animo' ? animo : k === 'energia' ? energia : estres}
                oninput={(e) => {
                  const n = Number(e.currentTarget.value);
                  if (k === 'animo') animo = n;
                  else if (k === 'energia') energia = n;
                  else estres = n;
                }}
              /></label
            >{/each}
        </div>
        <div class="acciones">
          <button class="primario" disabled={acciones.ocupado}>{textos.comun.guardar}</button
          ><button
            type="button"
            disabled={acciones.ocupado || !texto.trim()}
            onclick={() =>
              void acciones.hacer(async () => {
                const s = await diarioSugerir(texto);
                animo = s.animo;
                energia = s.energia;
                estres = s.estres;
                sugerida = true;
              }, false)}>{textos.ui.sugerir}</button
          >
        </div>
        {#if sugerida}<p class="ayuda">{textos.ui.sugerencia}</p>{/if}
      </form></Cajon
    >{/if}
  {#if registro === 'descanso'}<Cajon
      titulo={textos.ui.descanso}
      icono="reloj"
      oncerrar={() => (registro = null)}
      ><form
        class="registro-formulario"
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(() =>
            nocheAnotar({
              minutos: Math.round(horas * 60),
              despertar: minutos(despertar),
              pulso: pulso ?? null,
              pasos: pasos ?? null
            })
          );
        }}
      >
        <h2>{textos.ui.descanso}</h2>
        <div class="campos">
          <label
            >{textos.ui.horasSueno}<input
              type="number"
              bind:value={horas}
              min="0"
              max="16"
              step="0.1"
              required
            /></label
          ><label>{textos.ui.despertar}<input type="time" bind:value={despertar} required /></label
          ><label
            >{textos.ui.pulso}<input type="number" bind:value={pulso} min="25" max="220" /></label
          ><label
            >{textos.ui.pasos}<input type="number" bind:value={pasos} min="0" max="100000" /></label
          >
        </div>
        <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
      </form></Cajon
    >{/if}
  {#if registro === 'cafeina'}<Cajon
      titulo={textos.ui.cafeina}
      icono="mas"
      oncerrar={() => (registro = null)}
      ><form
        class="registro-formulario"
        onsubmit={(e) => {
          e.preventDefault();
          void acciones.hacer(async () => {
            await cafeinaTomar(fuente, mg);
            fuente = '';
          });
        }}
      >
        <h2>{textos.ui.cafeina}</h2>
        <div class="campos">
          <label>{textos.ui.fuente}<input bind:value={fuente} required maxlength="30" /></label
          ><label
            >{textos.ui.cafeinaMg}<input
              type="number"
              bind:value={mg}
              min="1"
              max="600"
              required
            /></label
          >
        </div>
        <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>{#if datos.cafeina}<p
            class="ayuda"
          >
            {numero(datos.cafeina.hoyMg)}
            {textos.unidades.mg} · {textos.comun.hoy}
          </p>{/if}
      </form></Cajon
    >{/if}
  <section class="panel">
    <h2>{textos.ui.historial} · {textos.ui.diario}</h2>
    <EstadoCarga
      cargando={diario.cargando}
      fallo={diario.fallo}
      reintentar={() => void cargar()}
    />{#if diario.datos && !diario.fallo}{#each diario.datos as entrada (entrada.id)}<div
          class="fila"
        >
          <div>
            <p class="detalle">{entrada.dia} · {entrada.hora}</p>
            <p class="nota">{entrada.texto}</p>
          </div>
          <BotonEliminar eliminar={() => diarioBorrar(entrada.id)} />
        </div>{:else}<EstadoVacio titulo={textos.comun.vacio} />{/each}{/if}
  </section>
{/if}

<style>
  .fecha {
    margin: 0 0 var(--sc-espacio-2);
    font-size: var(--sc-texto-sm);
    font-weight: 600;
    color: var(--m3c-primary);
  }
  h1 {
    font-size: var(--sc-texto-2xl);
    letter-spacing: -0.035em;
  }
  .perfil {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.8rem;
    height: 2.8rem;
    border-radius: var(--sc-radio-pleno);
    background: var(--m3c-surface-container-high);
    color: var(--m3c-on-surface);
    font-weight: 650;
    text-decoration: none;
    flex-shrink: 0;
  }
  .resumen-principal {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: var(--sc-espacio-4);
  }
  .protagonista {
    background: linear-gradient(
      125deg,
      var(--m3c-surface-container),
      color-mix(in srgb, var(--m3c-primary) 12%, var(--m3c-surface-container))
    );
  }
  .disposicion,
  .balance-datos {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-5);
    margin: var(--sc-espacio-4) 0;
  }
  .disposicion h2 {
    font-size: var(--sc-texto-xl);
    margin-bottom: var(--sc-espacio-2);
  }
  .ayuda {
    margin: var(--sc-espacio-2) 0;
  }
  .componentes-disposicion {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sc-espacio-4);
    margin-top: var(--sc-espacio-4);
  }
  .componentes-disposicion > div {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--sc-espacio-2);
    font-size: var(--sc-texto-xs);
  }
  .componentes-disposicion span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--m3c-on-surface-variant);
  }
  .componentes-disposicion small {
    font-weight: 400;
    color: var(--m3c-on-surface-variant);
  }
  .componentes-disposicion progress {
    width: 100%;
    grid-column: 1/-1;
    height: 0.25rem;
  }
  .sin-disposicion {
    padding: var(--sc-espacio-4) 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sc-espacio-2);
  }
  .sin-disposicion h2 {
    margin: 0;
  }
  .sin-disposicion > :global(svg) {
    color: var(--m3c-primary);
  }
  .balance-datos strong {
    font-size: var(--sc-texto-lg);
    font-variant-numeric: tabular-nums;
  }
  .balance-datos small {
    font-size: var(--sc-texto-xs);
    color: var(--m3c-on-surface-variant);
  }
  .balance .enlace-accion {
    width: 100%;
  }
  .secundarias {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .cuidados-rapidos {
    margin: var(--sc-espacio-5) 0;
  }
  .atajos {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--sc-espacio-3);
  }
  .atajos button {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: var(--sc-espacio-3);
    border-radius: var(--sc-radio-md);
    border-color: transparent;
    min-height: 3.7rem;
    background: var(--m3c-surface-container);
  }
  .atajos :global(svg) {
    flex-shrink: 0;
    color: var(--m3c-primary);
  }
  .resumen-ayuno {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sc-espacio-4);
    padding: var(--sc-espacio-4) var(--sc-espacio-5);
    margin: 0 0 var(--sc-espacio-5);
    border-radius: var(--sc-radio-lg);
    background: var(--m3c-surface-container);
    color: var(--m3c-on-surface);
    text-decoration: none;
  }
  .resumen-ayuno p {
    margin: var(--sc-espacio-1) 0 0;
    color: var(--m3c-on-surface-variant);
    font-weight: 400;
    font-size: var(--sc-texto-sm);
  }
  .nota {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .resumen-principal {
    grid-template-columns: 1fr;
  }
  :global(.carcasa:not([data-ancho='amplio'])) .atajos {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  :global(.carcasa[data-ancho='compacto']) .secundarias {
    grid-template-columns: 1fr;
  }
  :global(:root[data-papel='true']) .protagonista {
    background: var(--m3c-surface-container);
  }
</style>
