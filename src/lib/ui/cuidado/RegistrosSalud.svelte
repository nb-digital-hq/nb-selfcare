<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { nombreCatalogo } from '#lib/i18n/catalogo.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  let { datos }: { datos: ipc.ResumenCuidado } = $props();
  const acciones = new Acciones();
  let tipo = $state('');
  $effect(() => {
    if (!datos.aparatos.includes(tipo)) {
      tipo = datos.aparatos[0] ?? '';
      principal =
        tipo === 'oxygen' ? 98 : tipo === 'temperature' ? 36.5 : tipo === 'glucose' ? 90 : 120;
    }
  });
  let principal = $state(120);
  let secundario = $state<number | undefined>(80);
  let contexto = $state('random');
  let sintoma = $state('headache');
  let intensidad = $state(2);
  let notas = $state('');
  let fecha = $state(diaLocal());
  let nombre = $state('');
  let meses = $state(12);
  const bandas = () => textos.ui as Readonly<Record<string, string>>;
</script>

{#if datos.aparatos.length > 0}<details class="panel">
    <summary>{textos.ui.signos}</summary>
    <p class="ayuda">{textos.comun.estimacion}</p>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() =>
          ipc.vitalMedir(
            tipo,
            principal,
            tipo === 'blood_pressure' ? (secundario ?? null) : null,
            tipo === 'glucose' ? contexto : null
          )
        );
      }}
    >
      <div class="campos">
        <MenuSelector
          rotulo={textos.ui.tipo}
          bind:valor={tipo}
          opciones={Object.entries(textos.vitales)
            .filter(([clave]) => datos.aparatos.includes(clave))
            .map(([valor, etiqueta]) => ({ valor: valor as typeof tipo, etiqueta }))}
          onchange={(s) => {
            principal =
              s === 'oxygen' ? 98 : s === 'temperature' ? 36.5 : s === 'glucose' ? 90 : 120;
          }}
        /><label
          >{textos.ui.principal}<input
            type="number"
            bind:value={principal}
            min={tipo === 'blood_pressure'
              ? 60
              : tipo === 'oxygen'
                ? 50
                : tipo === 'temperature'
                  ? 32
                  : 20}
            max={tipo === 'blood_pressure'
              ? 260
              : tipo === 'oxygen'
                ? 100
                : tipo === 'temperature'
                  ? 43
                  : 600}
            step="0.1"
            required
          /></label
        >{#if tipo === 'blood_pressure'}<label
            >{textos.ui.secundario}<input
              type="number"
              bind:value={secundario}
              min="30"
              max="160"
              required
            /></label
          >{/if}{#if tipo === 'glucose'}<MenuSelector
            rotulo={textos.ui.contexto}
            bind:valor={contexto}
            opciones={(['fasting', 'after_meal', 'random'] as const).map((k) => ({
              valor: k as typeof contexto,
              etiqueta: textos.ui[k]
            }))}
          />{/if}
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>
    </form>
    {#each datos.lecturas as l (l.id)}<div class="fila">
        <div>
          <strong
            >{textos.vitales[l.tipo as keyof typeof textos.vitales] ?? textos.ui.signos}</strong
          >
          <p>
            {numero(l.principal, 1)}{l.secundario === null ? '' : ` / ${numero(l.secundario)}`} · {l.dia}
            {l.hora}
          </p>
          <p class="ayuda">{bandas()[l.banda] ?? textos.comun.estimacion}</p>
        </div>
      </div>{/each}
  </details>
{/if}
{#if datos.modulos.includes('symptoms')}<details class="panel">
    <summary>{textos.ui.sintomas}</summary>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(async () => {
          await ipc.sintomaRegistrar(sintoma, intensidad, notas || null);
          notas = '';
        });
      }}
    >
      <div class="campos">
        <MenuSelector
          rotulo={textos.ui.tipo}
          bind:valor={sintoma}
          opciones={datos.catalogoSintomas.map((s) => ({ valor: s, etiqueta: nombreCatalogo(s) }))}
        /><label
          >{textos.ui.nivel}<input
            type="number"
            bind:value={intensidad}
            min="1"
            max="5"
            required
          /></label
        ><label>{textos.ui.notas}<input bind:value={notas} maxlength="300" /></label>
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>
    </form>
    {#each datos.sintomas as s (s.id)}<div class="fila">
        <span>{nombreCatalogo(s.clave)} · {s.intensidad}/5 · {s.dia} {s.hora}</span>
      </div>{/each}
  </details>
{/if}
{#if datos.modulos.includes('checkups')}<details class="panel">
    <summary>{textos.ui.chequeos}</summary><label
      >{textos.ui.fecha}<input type="date" bind:value={fecha} max={diaLocal()} required /></label
    >{#each datos.chequeos as c (c.id)}<div class="fila">
        <div>
          <strong>{c.nombre ?? nombreCatalogo(c.clave)}</strong>
          <p class="detalle">{c.proximo ?? textos.ui.sinDato} · {c.meses} {textos.ui.meses}</p>
        </div>
        <div class="acciones">
          <button
            type="button"
            disabled={acciones.ocupado}
            onclick={() => void acciones.hacer(() => ipc.chequeoHecho(c.id, fecha))}
            >{textos.ui.realizado}</button
          ><button
            type="button"
            disabled={acciones.ocupado}
            onclick={() =>
              void acciones.hacer(() => ipc.chequeoAjustar(c.id, c.nombre, c.meses, !c.activo))}
            >{c.activo ? textos.comun.pausar : textos.comun.activar}</button
          >
        </div>
      </div>{/each}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(async () => {
          await ipc.chequeoAjustar(null, nombre, meses, true);
          nombre = '';
        });
      }}
    >
      <div class="campos">
        <label>{textos.ui.nombre}<input bind:value={nombre} maxlength="120" required /></label
        ><label
          >{textos.ui.meses}<input
            type="number"
            bind:value={meses}
            min="1"
            max="120"
            required
          /></label
        >
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
    </form>
  </details>
{/if}
