<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos, idioma } from '#lib/i18n/index.ts';
  import { textosExtra } from '#lib/i18n/sincronizacion.ts';
  const extra = $derived(textosExtra());
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones, Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import EstadoCarga from './EstadoCarga.svelte';
  import DialogoConfirmar from './DialogoConfirmar.svelte';
  import Icon from './Icon.svelte';

  const acciones = new Acciones();
  const estado = new Recurso<ipc.EstadoEnlace>();
  const conflictos = new Recurso<ipc.Conflicto[]>();
  let invitacion = $state<ipc.InvitacionEnlace | null>(null);
  let caduca = $state(0);
  let revocando = $state<ipc.ParEnlazado | null>(null);
  let reloj: ReturnType<typeof setInterval> | undefined;

  const cargar = () => {
    void estado.cargar(ipc.enlaceEstado);
    void conflictos.cargar(ipc.sincronizacionConflictos);
  };
  $effect(() => {
    void cambios.version;
    cargar();
  });
  onDestroy(() => {
    estado.cancelar();
    conflictos.cancelar();
    if (reloj) clearInterval(reloj);
  });

  const cuando = (iso: string | null) =>
    iso
      ? extra.enlace.ultima.replace(
          '{cuando}',
          new Intl.DateTimeFormat(idioma(), { dateStyle: 'medium', timeStyle: 'short' }).format(
            new Date(iso)
          )
        )
      : extra.enlace.nunca;

  const ruta = (qr: boolean[][]) =>
    qr.flatMap((fila, y) => fila.map((oscuro, x) => (oscuro ? `M${x} ${y}h1v1h-1z` : ''))).join('');

  async function invitar() {
    await acciones.hacer(async () => {
      invitacion = await ipc.enlaceInvitar();
      caduca = invitacion.caducaEnSegundos;
      if (reloj) clearInterval(reloj);
      reloj = setInterval(() => {
        caduca = Math.max(0, caduca - 1);
        if (caduca === 0) {
          invitacion = null;
          if (reloj) clearInterval(reloj);
          cargar();
        }
      }, 1000);
    }, false);
  }

  const etiquetaEntidad = (e: string) => (extra.entidades as Record<string, string>)[e] ?? e;
</script>

<section class="panel">
  <h2>{extra.enlace.titulo}</h2>
  <p class="ayuda">{extra.enlace.explicacion}</p>
  <EstadoCarga
    cargando={estado.cargando && !estado.datos}
    fallo={estado.fallo}
    reintentar={cargar}
  />
  {#if estado.datos}
    {@const e = estado.datos}
    <div class="fila-estado" role="status">
      <span class="punto" class:activo={e.activo} aria-hidden="true"></span>
      <span
        >{e.activo
          ? extra.enlace.activo.replace('{direccion}', e.direccion ?? '')
          : extra.enlace.inactivo}</span
      >
      {#if e.registros > 0}<span class="ayuda"
          >· {extra.enlace.registros.replace('{n}', String(e.registros))}</span
        >{/if}
    </div>
    <div class="acciones">
      {#if e.activo}
        <button type="button" class="primario" disabled={acciones.ocupado} onclick={invitar}
          ><Icon nombre="mas" tamano={18} />{invitacion
            ? extra.enlace.nuevoCodigo
            : extra.enlace.vincular}</button
        ><button
          type="button"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(async () => {
              invitacion = null;
              await ipc.enlaceActivar(false);
              cargar();
            }, false)}>{extra.enlace.desactivar}</button
        >
      {:else}
        <button
          type="button"
          class="primario"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(async () => {
              await ipc.enlaceActivar(true);
              cargar();
            }, false)}>{extra.enlace.activar}</button
        >
      {/if}
    </div>
    {#if invitacion && e.activo}
      <div class="invitacion">
        <svg
          class="qr"
          role="img"
          aria-label={extra.enlace.qrRotulo}
          viewBox={`-4 -4 ${invitacion.qr.length + 8} ${invitacion.qr.length + 8}`}
          shape-rendering="crispEdges"
          ><rect
            x="-4"
            y="-4"
            width={invitacion.qr.length + 8}
            height={invitacion.qr.length + 8}
            fill="var(--sc-qr-fondo)"
          /><path d={ruta(invitacion.qr)} fill="var(--sc-qr-tinta)" /></svg
        >
        <div>
          <p>{extra.enlace.qrAyuda}</p>
          <details class="codigo">
            <summary>{extra.enlace.sinCamara}</summary>
            <textarea readonly rows="4" aria-label={extra.enlace.codigoTexto}
              >{invitacion.texto}</textarea
            >
            <button
              type="button"
              onclick={() => void navigator.clipboard?.writeText(invitacion?.texto ?? '')}
              >{extra.enlace.copiarCodigo}</button
            >
          </details>
          <p class="ayuda" aria-live="polite">
            {Math.floor(caduca / 60)}:{String(caduca % 60).padStart(2, '0')}
          </p>
        </div>
      </div>
    {/if}

    <h3>{extra.enlace.telefonos}</h3>
    {#if e.pares.length === 0}<p class="ayuda">{extra.enlace.sinTelefonos}</p>{/if}
    <ul class="lista">
      {#each e.pares as par (par.nodo)}
        <li>
          <Icon nombre="telefono" tamano={18} />
          <div>
            <strong>{extra.enlace.telefono.replace('{alias}', par.alias)}</strong>
            <p class="ayuda">{cuando(par.ultima)}</p>
          </div>
          <button type="button" disabled={acciones.ocupado} onclick={() => (revocando = par)}
            >{extra.enlace.revocar}</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="panel">
  <h2>{extra.enlace.revisar}</h2>
  <p class="ayuda">{extra.enlace.revisarAyuda}</p>
  <EstadoCarga
    cargando={conflictos.cargando && !conflictos.datos}
    fallo={conflictos.fallo}
    reintentar={cargar}
  />
  {#if conflictos.datos?.length === 0}<p class="ayuda">{extra.enlace.sinConflictos}</p>{/if}
  {#each conflictos.datos ?? [] as c (c.nodo + c.operacion)}
    <article class="conflicto">
      <h3>{etiquetaEntidad(c.entidad)}</h3>
      {#if c.borradoAqui}<p class="ayuda">{extra.enlace.borradoAqui}</p>{/if}
      {#if c.borradoEnTelefono}<p class="ayuda">{extra.enlace.borradoTelefono}</p>{/if}
      {#if c.campos.length > 0}
        <table>
          <thead
            ><tr
              ><th scope="col">{extra.enlace.campo}</th><th scope="col"
                >{extra.enlace.esteEquipo}</th
              ><th scope="col">{extra.enlace.enTelefono}</th></tr
            ></thead
          >
          <tbody>
            {#each c.campos as campo (campo.campo)}
              <tr
                ><th scope="row">{campo.campo}</th><td>{campo.esteEquipo ?? '—'}</td><td
                  >{campo.telefono ?? '—'}</td
                ></tr
              >
            {/each}
          </tbody>
        </table>
      {/if}
      <div class="acciones">
        <button
          type="button"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(() => ipc.sincronizacionResolver(c.nodo, c.operacion, false))}
          >{extra.enlace.usarEste}</button
        ><button
          type="button"
          class="primario"
          disabled={acciones.ocupado}
          onclick={() =>
            void acciones.hacer(() => ipc.sincronizacionResolver(c.nodo, c.operacion, true))}
          >{extra.enlace.usarTelefono}</button
        >
      </div>
    </article>
  {/each}
</section>

<DialogoConfirmar
  abierto={revocando !== null}
  titulo={extra.enlace.revocar}
  mensaje={extra.enlace.revocarConfirmar}
  textoConfirmar={extra.enlace.revocar}
  destructivo
  ocupado={acciones.ocupado}
  onconfirmar={() => {
    const par = revocando;
    revocando = null;
    if (par)
      void acciones.hacer(async () => {
        await ipc.enlaceRevocar(par.nodo);
        cargar();
      });
  }}
  oncerrar={() => (revocando = null)}
/>

<style>
  .fila-estado {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-2);
    margin: var(--sc-espacio-3) 0;
  }
  .punto {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: var(--sc-radio-pleno);
    background: var(--m3c-outline);
  }
  .punto.activo {
    background: var(--m3c-primary);
  }
  .invitacion {
    display: flex;
    gap: var(--sc-espacio-5);
    align-items: center;
    margin: var(--sc-espacio-4) 0;
  }
  .codigo textarea {
    width: 100%;
    font-family: var(--sc-font-mono);
    font-size: var(--sc-texto-xs);
    margin: var(--sc-espacio-2) 0;
  }
  .qr {
    width: 13rem;
    height: 13rem;
    flex: none;
    border-radius: var(--sc-radio-md);
  }
  .lista {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .lista li {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-3) 0;
    border-bottom: 1px solid var(--m3c-outline-variant);
  }
  .lista li div {
    flex: 1;
  }
  .lista p {
    margin: 0;
  }
  .conflicto {
    border: 1px solid var(--m3c-outline-variant);
    border-radius: var(--sc-radio-md);
    padding: var(--sc-espacio-4);
    margin-top: var(--sc-espacio-4);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: var(--sc-espacio-3) 0;
    font-size: var(--sc-texto-sm);
  }
  th,
  td {
    text-align: start;
    padding: var(--sc-espacio-1) var(--sc-espacio-2);
    border-bottom: 1px solid var(--m3c-outline-variant);
    overflow-wrap: anywhere;
  }
</style>
