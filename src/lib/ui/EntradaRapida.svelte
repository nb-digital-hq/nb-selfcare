<script lang="ts">
  import { onMount } from 'svelte';
  import { textos } from '#lib/i18n/index.ts';
  import { textosExtra } from '#lib/i18n/sincronizacion.ts';
  import * as ipc from '#lib/core/ipc/index.ts';
  import { Acciones } from '#lib/core/recurso.svelte.ts';
  import { numero } from '#lib/core/dia.ts';
  import Icon from './Icon.svelte';
  /** Registro desde cualquier vista: comida en texto libre, agua o peso. */
  let { oncerrar }: { oncerrar: () => void } = $props();
  const t = $derived(textosExtra().rapida);
  const acciones = new Acciones();
  let dialogo: HTMLDialogElement | undefined = $state();
  let texto = $state('');
  let propuestas = $state<ipc.Propuesta[]>([]);
  const hora = new Date().getHours();
  let comida = $state<keyof typeof textos.comidas>(
    hora < 11 ? 'breakfast' : hora < 16 ? 'lunch' : hora < 21 ? 'dinner' : 'snack'
  );
  const numeroDe = (s: string) => Number(s.replace(',', '.'));
  const tipo = $derived.by(() => {
    const limpio = texto.trim().toLowerCase();
    const agua = limpio.match(/^(?:agua|water|água)\s+(\d{2,4})\s*(?:ml)?$/u);
    if (agua?.[1]) return { clase: 'agua' as const, valor: numeroDe(agua[1]) };
    const peso = limpio.match(/^(?:peso|weight)\s+(\d{2,3}(?:[.,]\d{1,2})?)\s*(?:kg)?$/u);
    if (peso?.[1]) return { clase: 'peso' as const, valor: numeroDe(peso[1]) };
    return limpio.length > 1 ? { clase: 'comida' as const, valor: 0 } : null;
  });
  let pedido = 0;
  $effect(() => {
    const frase = texto.trim();
    if (tipo?.clase !== 'comida') {
      propuestas = [];
      return;
    }
    const este = ++pedido;
    const espera = setTimeout(() => {
      void ipc
        .comidaProponer(frase)
        .then((p) => {
          if (este === pedido) propuestas = p;
        })
        .catch(() => {
          if (este === pedido) propuestas = [];
        });
    }, 220);
    return () => clearTimeout(espera);
  });
  const total = $derived(propuestas.reduce((s, p) => s + p.kcal, 0));
  onMount(() => dialogo?.showModal());
  async function guardar() {
    const actual = tipo;
    if (!actual) return;
    await acciones.hacer(async () => {
      if (actual.clase === 'agua') await ipc.comidaBeber(actual.valor);
      else if (actual.clase === 'peso') await ipc.cuerpoPesar(actual.valor);
      else await ipc.comidaRegistrarFrase(texto.trim(), comida);
      dialogo?.close();
    });
  }
</script>

<dialog
  bind:this={dialogo}
  class="entrada-rapida"
  aria-labelledby="rapida-titulo"
  onclose={oncerrar}
  onclick={(e) => {
    if (e.target === dialogo) dialogo?.close();
  }}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void guardar();
    }}
  >
    <div class="campo">
      <Icon nombre="mas" tamano={20} />
      <label class="sr-only" for="rapida-texto" id="rapida-titulo">{t.titulo}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="rapida-texto"
        bind:value={texto}
        maxlength="200"
        placeholder={t.marcador}
        autocomplete="off"
        autofocus
      />
      <kbd>{t.tecla}</kbd>
    </div>
    {#if tipo?.clase === 'comida'}
      <div class="momentos" role="group" aria-label={textos.ui.momento}>
        {#each Object.entries(textos.comidas) as [clave, nombre]}<button
            type="button"
            class:seleccionado={comida === clave}
            aria-pressed={comida === clave}
            onclick={() => (comida = clave as keyof typeof textos.comidas)}>{nombre}</button
          >{/each}
      </div>
      <ul class="propuestas" aria-live="polite">
        {#each propuestas as p}
          <li>
            <span>{p.alimento?.nombre ?? p.leido.termino}</span>
            <small
              >{numero(p.leido.gramos)}
              {textos.unidades.g} · {numero(p.kcal)}
              {textos.unidades.kcal}</small
            >
          </li>
        {/each}
        {#if propuestas.some((p) => !p.alimento)}<li class="nota">{t.sinCoincidencia}</li>{/if}
      </ul>
    {:else if tipo}
      <p class="resumen" aria-live="polite">
        {(tipo.clase === 'agua' ? t.agua : t.peso).replace('{n}', numero(tipo.valor, 1))}
      </p>
    {/if}
    <footer>
      <small>{t.ayuda}</small>
      {#if tipo?.clase === 'comida' && total > 0}<strong
          >{numero(total)} {textos.unidades.kcal}</strong
        >{/if}
      <button class="primario" disabled={!tipo || acciones.ocupado}>{t.guardar}</button>
    </footer>
  </form>
</dialog>

<style>
  .entrada-rapida {
    width: min(40rem, calc(100vw - 2rem));
    margin: 12vh auto auto;
    padding: 0;
    border: 1px solid var(--m3c-outline-variant);
    border-radius: var(--sc-radio-xl);
    background: var(--m3c-surface-container-high);
    color: var(--m3c-on-surface);
  }
  .entrada-rapida::backdrop {
    background: color-mix(in srgb, var(--m3c-scrim) 45%, transparent);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-4) var(--sc-espacio-5) var(--sc-espacio-4);
  }
  .campo {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    color: var(--m3c-primary);
  }
  .campo input {
    flex: 1;
    border: none;
    background: none;
    padding: var(--sc-espacio-2) 0;
    font-size: var(--sc-texto-lg);
    color: var(--m3c-on-surface);
  }
  .campo:focus-within {
    box-shadow: 0 1px 0 var(--m3c-primary);
  }
  kbd {
    font: inherit;
    font-size: var(--sc-texto-xs);
    color: var(--m3c-on-surface-variant);
    border: 1px solid var(--m3c-outline-variant);
    border-radius: var(--sc-radio-sm);
    padding: 0.1rem 0.4rem;
  }
  .momentos {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sc-espacio-2);
  }
  .momentos button.seleccionado {
    background: var(--m3c-secondary-container);
    color: var(--m3c-on-secondary-container);
  }
  .propuestas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--sc-espacio-1);
  }
  .propuestas li {
    display: flex;
    justify-content: space-between;
    gap: var(--sc-espacio-3);
    padding: var(--sc-espacio-2) var(--sc-espacio-3);
    border-radius: var(--sc-radio-md);
    background: var(--m3c-surface-container);
  }
  .propuestas small,
  .nota,
  footer small {
    color: var(--m3c-on-surface-variant);
  }
  .propuestas .nota {
    background: none;
    font-size: var(--sc-texto-sm);
  }
  .resumen {
    margin: 0;
    font-size: var(--sc-texto-lg);
  }
  footer {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-3);
    border-top: 1px solid var(--m3c-outline-variant);
    padding-top: var(--sc-espacio-3);
  }
  footer small {
    flex: 1;
  }
</style>
