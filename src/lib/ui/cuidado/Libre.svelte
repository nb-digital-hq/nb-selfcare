<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import { numero } from '#lib/core/dia.ts';
  import BotonEliminar from '../BotonEliminar.svelte';
  import DialogoConfirmar from '../DialogoConfirmar.svelte';
  let { datos }: { datos: ipc.ResumenCuidado } = $props();
  const acciones = new Acciones();
  let sustancia = $state('beer');
  let unidades = $state(1);
  let reiniciar = $state<number | null>(null);
  let clave = $state('tobacco');
  let nombre = $state('');
  let haceDias = $state(0);
  let motivo = $state('');
</script>

{#if datos.modulos.includes('substances')}<details class="panel">
    <summary>{textos.ui.consumo}</summary>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => ipc.sustanciaConsumir(sustancia, unidades));
      }}
    >
      <div class="campos">
        <MenuSelector
          rotulo={textos.ui.tipo}
          bind:valor={sustancia}
          opciones={(['beer', 'wine', 'spirit', 'cocktail', 'cigarette', 'vape'] as const).map(
            (k) => ({ valor: k as typeof sustancia, etiqueta: textos.ui[k] })
          )}
        /><label
          >{textos.ui.unidades}<input
            type="number"
            bind:value={unidades}
            min="1"
            max="100"
            required
          /></label
        >
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.registrar}</button>
    </form>
    <p class="ayuda">
      {textos.ui.semana}: {numero(datos.consumo.tragosSemana, 1)} · {textos.sustancias.alcohol} / {datos
        .consumo.tabacoSemana} · {textos.sustancias.tobacco}
    </p>
    {#each datos.consumo.ultimos as c (c.id)}<div class="fila">
        <span
          >{textos.ui[c.clave as 'beer' | 'wine' | 'spirit' | 'cocktail' | 'cigarette' | 'vape'] ??
            textos.ui.tipo} · {c.unidades} · {c.dia}</span
        ><BotonEliminar eliminar={() => ipc.sustanciaBorrar(c.id)} />
      </div>{/each}
  </details>
{/if}
{#if datos.modulos.includes('quit')}<details class="panel">
    <summary>{textos.ui.libre}</summary>{#each datos.contadores as c (c.id)}<div class="resultado">
        <h3>
          {c.nombre ?? textos.libres[c.clave as keyof typeof textos.libres] ?? textos.ui.libre}
        </h3>
        <p>{textos.ui.fecha}: {c.inicio.slice(0, 10)} · {textos.ui.ganas}: {c.ganasSuperadas}</p>
        <div class="acciones">
          <button
            type="button"
            disabled={acciones.ocupado}
            onclick={() => void acciones.hacer(() => ipc.libreGanas(c.id))}
            >{textos.ui.ganas}</button
          ><button
            type="button"
            disabled={acciones.ocupado}
            onclick={() => void acciones.hacer(() => ipc.libreCelebrar(c.id))}
            >{textos.ui.celebrar}</button
          ><button type="button" onclick={() => (reiniciar = c.id)}>{textos.ui.reiniciar}</button
          ><BotonEliminar
            eliminar={() => ipc.libreQuitar(c.id, true)}
            archivar={() => ipc.libreQuitar(c.id, false)}
          />
        </div>
      </div>{/each}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(async () => {
          await ipc.libreCrear({
            clave,
            nombre: nombre || null,
            haceDias,
            costeDiario: null,
            unidadesDiarias: null,
            minutosDiarios: null,
            motivo: motivo || null
          });
          nombre = '';
          motivo = '';
        });
      }}
    >
      <h3>{textos.ui.contador}</h3>
      <div class="campos">
        <MenuSelector
          rotulo={textos.ui.tipo}
          bind:valor={clave}
          opciones={Object.entries(textos.libres).map(([valor, etiqueta]) => ({
            valor: valor as typeof clave,
            etiqueta
          }))}
        /><label
          >{textos.ui.nombre}<input
            bind:value={nombre}
            maxlength="80"
            required={clave === 'custom'}
          /></label
        ><label
          >{textos.ui.haceDias}<input
            type="number"
            bind:value={haceDias}
            min="0"
            max="3650"
            required
          /></label
        ><label>{textos.ui.motivo}<input bind:value={motivo} maxlength="300" /></label>
      </div>
      <button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
    </form>
  </details>
{/if}
<DialogoConfirmar
  abierto={reiniciar !== null}
  titulo={textos.ui.reiniciar}
  mensaje={textos.ui.reinicioMensaje}
  ocupado={acciones.ocupado}
  oncerrar={() => (reiniciar = null)}
  onconfirmar={() =>
    void acciones.hacer(async () => {
      if (reiniciar !== null) await ipc.libreReiniciar(reiniciar);
      reiniciar = null;
    })}
/>
