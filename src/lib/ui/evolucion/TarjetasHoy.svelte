<script lang="ts">
  import { onDestroy } from 'svelte';
  import { textos, idioma } from '#lib/i18n/index.ts';
  import { textosExtra } from '#lib/i18n/sincronizacion.ts';
  const extra = $derived(textosExtra());
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Recurso } from '#lib/core/recurso.svelte.ts';
  import { cambios } from '#lib/core/cambios.svelte.ts';
  import { diaLocal, numero } from '#lib/core/dia.ts';
  import Icon from '../Icon.svelte';
  /** Hoy en el escritorio: lo que llegó del teléfono y cómo va la semana. */
  const enlace = new Recurso<ipc.EstadoEnlace>();
  const evolucion = new Recurso<ipc.Evolucion>();
  const deHoy = new Recurso<ipc.Movimiento[]>();
  const hoy = diaLocal();
  $effect(() => {
    void cambios.version;
    void enlace.cargar(ipc.enlaceEstado);
    void evolucion.cargar(() => ipc.evolucion(30));
    void deHoy.cargar(() => ipc.historial(hoy, hoy));
  });
  onDestroy(() => {
    enlace.cancelar();
    evolucion.cancelar();
    deHoy.cancelar();
  });
  const PRIORIDAD: ipc.Medida[] = ['pesoKg', 'suenoH', 'aguaMl', 'kcal', 'animo', 'pasos'];
  const destacadas = $derived(
    PRIORIDAD.map((m) => evolucion.datos?.semana.find((c) => c.medida === m))
      .filter((c): c is ipc.Comparacion => !!c && c.esta !== null)
      .slice(0, 3)
  );
  /** Cuántas medidas tienen dato cada día: la intensidad de la cuadrícula del mes. */
  const MEDIDAS: (keyof ipc.DiaEvolucion)[] = [
    'kcal',
    'aguaMl',
    'pesoKg',
    'suenoH',
    'pasos',
    'animo',
    'entrenoMin',
    'habitos',
    'cafeinaMg'
  ];
  const celdas = $derived(
    (evolucion.datos?.dias ?? []).map((d) => ({
      dia: d.dia,
      n: MEDIDAS.filter((m) => d[m] !== null).length
    }))
  );
  const activos = $derived(celdas.filter((c) => c.n > 0).length);
  const nivel = (n: number) => (n === 0 ? 0 : n < 3 ? 1 : n < 5 ? 2 : 3);
  const ICONO: Record<ipc.TipoDeMovimiento, 'comida' | 'cuerpo' | 'cuidado' | 'editar' | 'mas'> = {
    comida: 'comida',
    agua: 'mas',
    peso: 'cuerpo',
    entreno: 'cuerpo',
    diario: 'editar',
    cafeina: 'mas',
    sintoma: 'cuidado',
    signo: 'cuidado',
    digestion: 'cuidado',
    sesion: 'cuidado'
  };
  const cuando = (iso: string | null) =>
    iso
      ? new Intl.DateTimeFormat(idioma(), { dateStyle: 'medium', timeStyle: 'short' }).format(
          new Date(iso)
        )
      : null;
  /** Pasos, energía, agua, hábitos y cafeína se leen en enteros; el resto con un decimal. */
  const dec = (m: ipc.Medida) =>
    ['pasos', 'kcal', 'aguaMl', 'habitos', 'cafeinaMg'].includes(m) ? 0 : 1;
</script>

<div class="tarjetas-hoy">
  <section class="panel">
    <div class="rotulo-dato">
      <Icon nombre="telefono" tamano={18} />{extra.enlace.desdeTelefono}
    </div>
    {#if enlace.datos && enlace.datos.pares.length > 0}
      {@const ultima = cuando(enlace.datos.ultima)}
      <p>{ultima ? extra.enlace.ultima.replace('{cuando}', ultima) : extra.enlace.nunca}</p>
      <p class="ayuda">
        {enlace.datos.porRevisar > 0
          ? extra.enlace.porRevisar.replace('{n}', String(enlace.datos.porRevisar))
          : extra.enlace.alDia}
      </p>
      {#if enlace.datos.porRevisar > 0}<a class="enlace-discreto" href="/ajustes?seccion=telefono"
          >{extra.enlace.revisar}<Icon nombre="derecha" tamano={16} /></a
        >{/if}
    {:else}
      <p class="ayuda">{extra.enlace.desdeTelefonoSub}</p>
      <a class="enlace-accion" href="/ajustes?seccion=telefono"
        ><Icon nombre="mas" tamano={18} />{extra.enlace.conectarTelefono}</a
      >
    {/if}
  </section>
  <section class="panel">
    <div class="rotulo-dato"><Icon nombre="lista" tamano={18} />{extra.evolucion.semana}</div>
    {#if destacadas.length === 0}<p class="ayuda">{extra.evolucion.sinDatosSemana}</p>{/if}
    <dl class="semana">
      {#each destacadas as c (c.medida)}
        {@const delta = c.anterior === null || c.esta === null ? null : c.esta - c.anterior}
        <div>
          <dt>{extra.evolucion[c.medida]}</dt>
          <dd>
            {numero(c.esta, dec(c.medida))}{#if delta !== null && Math.abs(delta) > 0.05}<small
                >{delta > 0 ? ' ▲' : ' ▼'} {numero(Math.abs(delta), dec(c.medida))}</small
              >{/if}
          </dd>
        </div>
      {/each}
    </dl>
    {#if celdas.length}
      <p class="ayuda">{extra.evolucion.mesActivos.replace('{n}', String(activos))}</p>
      <div class="mes" role="list">
        {#each celdas as c (c.dia)}<span
            role="listitem"
            class={`celda n${nivel(c.n)}`}
            title={c.dia}
            aria-label={`${c.dia}: ${c.n}`}
          ></span>{/each}
      </div>
    {/if}
    <a class="enlace-discreto" href="/perfil?seccion=semana"
      >{extra.evolucion.verSemana}<Icon nombre="derecha" tamano={16} /></a
    >
  </section>
  <section class="panel hoy-lista">
    <div class="rotulo-dato"><Icon nombre="reloj" tamano={18} />{extra.evolucion.loDeHoy}</div>
    <p class="ayuda">{extra.evolucion.loDeHoySub}</p>
    {#if deHoy.datos?.length === 0}<p class="ayuda">{extra.evolucion.sinHoy}</p>{/if}
    <ol>
      {#each (deHoy.datos ?? []).slice(0, 8) as m (m.tipo + m.id)}
        <li>
          <time>{m.instante.slice(11, 16)}</time>
          <Icon nombre={ICONO[m.tipo]} tamano={16} />
          <span>{extra.evolucion[m.tipo]}{m.titulo ? ` · ${m.titulo}` : ''}</span>
          <small>{numero(m.valor, 1)}</small>
        </li>
      {/each}
    </ol>
  </section>
</div>

<style>
  .tarjetas-hoy {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(22rem, 100%), 1fr));
    gap: var(--sc-espacio-4);
    margin-bottom: var(--sc-espacio-5);
  }
  .tarjetas-hoy .panel {
    margin-bottom: 0;
  }
  .hoy-lista {
    grid-column: 1 / -1;
  }
  .hoy-lista ol {
    list-style: none;
    margin: var(--sc-espacio-2) 0 0;
    padding: 0;
    display: grid;
    gap: var(--sc-espacio-1);
  }
  .hoy-lista li {
    display: grid;
    grid-template-columns: 3rem 1.25rem 1fr auto;
    align-items: center;
    gap: var(--sc-espacio-2);
    padding: var(--sc-espacio-2) 0;
    border-bottom: 1px solid color-mix(in srgb, var(--m3c-outline-variant) 55%, transparent);
  }
  .hoy-lista time,
  .hoy-lista small {
    color: var(--m3c-on-surface-variant);
    font-variant-numeric: tabular-nums;
    font-size: var(--sc-texto-sm);
  }
  .hoy-lista span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mes {
    display: grid;
    grid-template-columns: repeat(15, 0.9rem);
    gap: 0.25rem;
    margin: var(--sc-espacio-2) 0;
  }
  .celda {
    width: 0.9rem;
    height: 0.9rem;
    border-radius: 0.2rem;
    background: var(--m3c-surface-container-highest);
  }
  .celda.n1 {
    background: color-mix(in srgb, var(--m3c-primary) 35%, var(--m3c-surface-container-highest));
  }
  .celda.n2 {
    background: color-mix(in srgb, var(--m3c-primary) 65%, var(--m3c-surface-container-highest));
  }
  .celda.n3 {
    background: var(--m3c-primary);
  }
  .semana {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--sc-espacio-3);
    margin: var(--sc-espacio-3) 0;
  }
  .semana dt {
    color: var(--m3c-on-surface-variant);
    font-size: var(--sc-texto-sm);
  }
  .semana dd {
    margin: 0;
    font-size: var(--sc-texto-lg);
    font-weight: 600;
  }
  .semana small {
    font-size: var(--sc-texto-xs);
    color: var(--m3c-on-surface-variant);
  }
</style>
