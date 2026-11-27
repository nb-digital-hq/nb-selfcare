<script lang="ts">
  import MenuSelector from '#lib/ui/MenuSelector.svelte';
  import { textos } from '#lib/i18n/index.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import { hora, minutos, numero } from '#lib/core/dia.ts';
  import DiasSemana from '../DiasSemana.svelte';
  import BotonEliminar from '../BotonEliminar.svelte';
  let { datos }: { datos: ipc.ResumenCuidado } = $props();
  const acciones = new Acciones();
  let editando = $state<number | null>(null);
  let horarios = $state('08:00');
  let reponer = $state(30);
  const nuevo = (): ipc.BorradorTratamiento => ({
    nombre: '',
    dosis: '',
    tipo: 'medication',
    horas: [480],
    dias: 127,
    reserva: null,
    unidadesPorToma: 1,
    avisos: true,
    bajaGlucosa: false,
    retieneAgua: false,
    notas: null
  });
  let b = $state(nuevo());
  function editar(t: ipc.Tratamiento) {
    editando = t.id;
    b = {
      nombre: t.nombre,
      dosis: t.dosis,
      tipo: t.tipo,
      horas: [...t.horas],
      dias: t.dias,
      reserva: t.reserva,
      unidadesPorToma: t.unidadesPorToma,
      avisos: t.avisos,
      bajaGlucosa: t.bajaGlucosa,
      retieneAgua: t.retieneAgua,
      notas: t.notas
    };
    horarios = t.horas.map(hora).join(', ');
  }
  async function guardar() {
    const partes = horarios
      .split(',')
      .map((x) => x.trim())
      .filter(Boolean);
    if (partes.some((x) => !/^([01]\d|2[0-3]):[0-5]\d$/.test(x)))
      throw new ipc.ScError('INVALID_INPUT');
    await ipc.tratamientoGuardar(editando, { ...$state.snapshot(b), horas: partes.map(minutos) });
    b = nuevo();
    editando = null;
    horarios = '08:00';
  }
</script>

<section class="panel">
  <h2>{textos.ui.tomas}</h2>
  <p class="ayuda">{textos.ui.sinPremios}</p>
  {#each datos.tomas as t (`${t.tratamiento}:${t.hora}`)}<div class="fila">
      <div>
        <strong>{t.nombre}</strong>
        <p class="detalle">
          {t.dosis} · {t.hora === null ? '—' : hora(t.hora)} · {t.estado === 'taken'
            ? textos.ui.tomada
            : t.estado === 'skipped'
              ? textos.ui.omitida
              : textos.ui.pendiente}
        </p>
      </div>
      <div class="acciones">
        <button
          type="button"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(() => ipc.tomaMarcar(t.tratamiento, t.hora, true), false)}
          >{textos.ui.tomada}</button
        ><button
          type="button"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(() => ipc.tomaMarcar(t.tratamiento, t.hora, false), false)}
          >{textos.ui.omitida}</button
        >{#if t.estado !== 'pending'}<button
            type="button"
            disabled={acciones.ocupado}
            onclick={() =>
              void acciones.hacer(() => ipc.tomaMarcar(t.tratamiento, t.hora, null), false)}
            >{textos.ui.deshacer}</button
          >{/if}
      </div>
    </div>{/each}
</section>
<details class="panel">
  <summary>{textos.ui.tratamientos}</summary>
  {#each datos.tratamientos as t (t.id)}<div class="fila">
      <div>
        <strong>{t.nombre}</strong>
        <p class="detalle">
          {t.dosis} · {t.horas.map(hora).join(', ')} · {t.reserva === null
            ? '—'
            : numero(t.reserva, 1)}
        </p>
      </div>
      <div class="acciones">
        <button type="button" onclick={() => editar(t)}>{textos.comun.editar}</button><button
          type="button"
          disabled={acciones.ocupado}
          onclick={() => void acciones.hacer(() => ipc.tratamientoEstado(t.id, !t.activo))}
          >{t.activo ? textos.comun.pausar : textos.comun.reanudar}</button
        ><BotonEliminar eliminar={() => ipc.tratamientoEstado(t.id, null)} />
      </div>
    </div>
    <form
      class="acciones"
      onsubmit={(e) => {
        e.preventDefault();
        void acciones.hacer(() => ipc.tratamientoReponer(t.id, reponer));
      }}
    >
      <label
        >{textos.ui.reponer}<input
          type="number"
          bind:value={reponer}
          min="1"
          max="10000"
          step="0.1"
          required
        /></label
      ><button disabled={acciones.ocupado}>{textos.comun.guardar}</button>
    </form>{/each}
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void acciones.hacer(guardar);
    }}
  >
    <h3>{editando === null ? textos.comun.registrar : textos.comun.editar}</h3>
    <div class="campos">
      <label>{textos.ui.nombre}<input bind:value={b.nombre} maxlength="120" required /></label
      ><label>{textos.ui.dosis}<input bind:value={b.dosis} maxlength="120" required /></label
      ><MenuSelector
        rotulo={textos.ui.tipo}
        bind:valor={b.tipo}
        opciones={[
          { valor: 'medication' as typeof b.tipo, etiqueta: textos.ui.medicamento },
          { valor: 'supplement' as typeof b.tipo, etiqueta: textos.ui.suplemento }
        ]}
      /><label>{textos.ui.horarios}<input bind:value={horarios} maxlength="150" /></label><label
        >{textos.ui.reserva}<input
          type="number"
          min="0"
          max="100000"
          step="0.1"
          value={b.reserva ?? ''}
          oninput={(e) =>
            (b.reserva = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
        /></label
      ><label
        >{textos.ui.unidadesPorToma}<input
          type="number"
          min="0.1"
          max="100"
          step="0.1"
          bind:value={b.unidadesPorToma}
          required
        /></label
      >
    </div>
    <DiasSemana bind:valor={b.dias} /><label class="check"
      ><input type="checkbox" bind:checked={b.avisos} />{textos.ui.avisar}</label
    ><label class="check"
      ><input type="checkbox" bind:checked={b.bajaGlucosa} />{textos.ui.hipoglucemiantes}</label
    ><label class="check"
      ><input type="checkbox" bind:checked={b.retieneAgua} />{textos.ui.retieneAgua}</label
    >
    <div class="acciones">
      <button class="primario" disabled={acciones.ocupado}>{textos.comun.guardar}</button
      >{#if editando !== null}<button
          type="button"
          onclick={() => {
            editando = null;
            b = nuevo();
            horarios = '08:00';
          }}>{textos.comun.cancelar}</button
        >{/if}
    </div>
  </form>
</details>
