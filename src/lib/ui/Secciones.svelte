<script lang="ts">
  let {
    clave,
    opciones,
    actual = $bindable(),
    etiqueta
  }: {
    clave: string;
    opciones: readonly { id: string; titulo: string }[];
    actual: string;
    etiqueta: string;
  } = $props();
  function teclado(e: KeyboardEvent, indice: number) {
    let siguiente = indice;
    if (e.key === 'ArrowRight') siguiente = (indice + 1) % opciones.length;
    else if (e.key === 'ArrowLeft') siguiente = (indice + opciones.length - 1) % opciones.length;
    else if (e.key === 'Home') siguiente = 0;
    else if (e.key === 'End') siguiente = opciones.length - 1;
    else return;
    e.preventDefault();
    const opcion = opciones[siguiente];
    if (!opcion) return;
    actual = opcion.id;
    document.getElementById(`${clave}-${actual}`)?.focus();
  }
</script>

<div class="secciones" role="tablist" aria-label={etiqueta}>
  {#each opciones as opcion, indice}<button
      id={`${clave}-${opcion.id}`}
      type="button"
      role="tab"
      aria-selected={actual === opcion.id}
      aria-controls={`${clave}-contenido`}
      tabindex={actual === opcion.id ? 0 : -1}
      onclick={() => (actual = opcion.id)}
      onkeydown={(e) => teclado(e, indice)}>{opcion.titulo}</button
    >{/each}
</div>

<style>
  .secciones {
    display: flex;
    align-items: center;
    gap: var(--sc-espacio-2);
    flex-wrap: wrap;
    padding-bottom: var(--sc-espacio-3);
    margin-bottom: var(--sc-espacio-4);
    border-bottom: 1px solid var(--m3c-outline-variant);
    scrollbar-width: thin;
  }
  button {
    background: transparent;
    border: 1px solid transparent;
    font-weight: 450;
    white-space: nowrap;
    min-height: 2.5rem;
  }
  button[aria-selected='true'] {
    background: var(--m3c-secondary-container);
    color: var(--m3c-on-secondary-container);
    font-weight: 650;
    border-color: var(--m3c-outline-variant);
  }
</style>
