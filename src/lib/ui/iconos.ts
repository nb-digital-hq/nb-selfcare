/**
 * El catálogo de iconos de NB SelfCare.
 *
 * El mismo lenguaje visual que NB Books, para que las dos aplicaciones se lean
 * como una familia (ADR-008): rejilla de 24 con área segura de 2 a 22, trazo de
 * 1,5 con extremos y uniones redondeados, sin relleno y sin color propio. Los que
 * ya existen en NB Books se copian tal cual; los propios de SelfCare siguen sus
 * mismas medidas (círculos de r = 8,5 centrados, contenedores de 3,5 a 20,5).
 *
 * **Nunca hay emojis ni letras haciendo de icono.** La lista es cerrada: un icono
 * que falta obliga a decidir qué significa antes de dibujarlo.
 */

/** Nombre de un icono del catálogo. */
export type NombreDeIcono = keyof typeof ICONOS;

/** El contenido de cada `<svg>`, sin la etiqueta: la pone `Icon.svelte`. */
export const ICONOS = {
  telefono: '<rect x="7" y="2.5" width="10" height="19" rx="2"/><path d="M11 18.5h2"/>',
  comida: '<path d="M5 3v7a3 3 0 0 0 6 0V3M8 3v18M19 21V3c-4 2-4 9 0 10"/>',
  cuerpo: '<path d="M7 4h10l3 16H4z"/><path d="M9 8a3 3 0 0 1 6 0M12 8l1-2"/>',
  cuidado:
    '<path d="M12 20S3 15 3 8a5 5 0 0 1 9-3 5 5 0 0 1 9 3c0 7-9 12-9 12Z"/><path d="M7 11h3l1-3 2 6 1-3h3"/>',
  nibi: '<path d="m6 5 2-3 3 4h4l3-3 1 6c3 6-1 12-7 12S2 16 5 10Z"/><circle cx="9" cy="11" r=".5"/><circle cx="15" cy="11" r=".5"/><path d="M10 16h4"/>',

  abajo: '<path d="m5.5 8.75 6.5 6.5 6.5-6.5"/>',
  advertencia: '<path d="M12 3.5 20.75 20.5H3.25z"/><path d="M12 10v4.25M12 17.5h.01"/>',
  ajustes:
    '<circle cx="12" cy="12" r="3.25"/><path d="M12 2.5v2.25M12 19.25v2.25M21.5 12h-2.25M4.75 12H2.5M18.72 5.28l-1.6 1.6M6.88 17.12l-1.6 1.6M18.72 18.72l-1.6-1.6M6.88 6.88l-1.6-1.6"/>',
  almacenamiento:
    '<ellipse cx="12" cy="6.25" rx="7.5" ry="2.75"/><path d="M4.5 6.25v11.5c0 1.52 3.36 2.75 7.5 2.75s7.5-1.23 7.5-2.75V6.25"/><path d="M4.5 12c0 1.52 3.36 2.75 7.5 2.75s7.5-1.23 7.5-2.75"/>',
  anterior: '<path d="M6 5v14"/><path d="M18.5 5.5 9 12l9.5 6.5z"/>',
  apariencia: '<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5v17"/>',
  arriba: '<path d="m5.5 15.25 6.5-6.5 6.5 6.5"/>',
  biblioteca:
    '<path d="M3.5 4.5h4.25v15H3.5zM9.75 4.5H14v15H9.75z"/><path d="m16.15 5.6 3.7 1.05-3.4 12.85-3.7-1.05z"/>',
  borrar:
    '<path d="M3.5 6.5h17M9.5 6.5V4.5a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v2"/><path d="m6 6.5 1.1 13.1a1.5 1.5 0 0 0 1.5 1.4h6.8a1.5 1.5 0 0 0 1.5-1.4L18 6.5"/><path d="M10.25 10.5v6.25M13.75 10.5v6.25"/>',
  buscar: '<circle cx="10.75" cy="10.75" r="6.75"/><path d="m15.55 15.55 4.95 4.95"/>',
  cargando: '<path d="M12 3.5a8.5 8.5 0 1 1-6.01 2.49"/>',
  carpeta:
    '<path d="M3.5 6.5A1.5 1.5 0 0 1 5 5h4.25l2 2.5H19A1.5 1.5 0 0 1 20.5 9v9a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 18z"/>',
  cerrar: '<path d="m6.5 6.5 11 11M17.5 6.5l-11 11"/>',
  cola: '<path d="M3.5 6h13M3.5 10.5h13M3.5 15h7"/><path d="m14.5 13.5 6 3.5-6 3.5z"/>',
  copiar:
    '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M15.5 8.5V6.5a2 2 0 0 0-2-2h-7a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h2"/>',
  comprobado: '<path d="m3.5 12.5 5.5 5.5L20.5 6.5"/>',
  derecha: '<path d="m8.75 5.5 6.5 6.5-6.5 6.5"/>',
  descargar:
    '<path d="M12 3.5v12"/><path d="m6.75 10.25 5.25 5.25 5.25-5.25"/><path d="M4.5 20.5h15"/>',
  editar:
    '<path d="M4.5 19.5h3.25l9.75-9.75-3.25-3.25L4.5 16.25z"/><path d="m14.25 6.5 3.25 3.25"/><path d="m16.75 4 3.25 3.25-2.5 2.5"/>',
  error: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5v5.25M12 16h.01"/>',
  exportar:
    '<path d="M12 14V3.5M8 7.25l4-3.75 4 3.75"/><path d="M3.5 16.5v2.5a1.5 1.5 0 0 0 1.5 1.5h14a1.5 1.5 0 0 0 1.5-1.5v-2.5"/>',
  favorito:
    '<path d="M12 19.5S3.5 14.6 3.5 8.9A4.4 4.4 0 0 1 12 7.1a4.4 4.4 0 0 1 8.5 1.8c0 5.7-8.5 10.6-8.5 10.6Z"/>',
  importar:
    '<path d="M12 3.5v10.5M8 10.25l4 4 4-4"/><path d="M3.5 16.5v2.5a1.5 1.5 0 0 0 1.5 1.5h14a1.5 1.5 0 0 0 1.5-1.5v-2.5"/>',
  informacion: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11.25v5.25M12 8h.01"/>',
  inicio:
    '<path d="M3.5 11.2 12 4l8.5 7.2"/><path d="M5.75 9.9V19a1.5 1.5 0 0 0 1.5 1.5h9.5a1.5 1.5 0 0 0 1.5-1.5V9.9"/><path d="M9.75 20.5v-5.75h4.5v5.75"/>',
  izquierda: '<path d="m15.25 5.5-6.5 6.5 6.5 6.5"/>',
  lista:
    '<path d="M8.5 6.5h12M8.5 12h12M8.5 17.5h12"/><path d="M3.5 6.5h1.25M3.5 12h1.25M3.5 17.5h1.25"/>',
  mas: '<path d="M12 3.5v17M3.5 12h17"/>',
  masHorizontal:
    '<circle cx="12" cy="12" r="1.5"/><circle cx="5" cy="12" r="1.5"/><circle cx="19" cy="12" r="1.5"/>',
  menu: '<path d="M3.5 7h17M3.5 12h17M3.5 17h17"/>',
  pausa: '<path d="M9 4.5v15M15 4.5v15"/>',
  privacidad:
    '<path d="M12 3.5 5 6.25v6.15c0 4.2 2.9 7.35 7 8.6 4.1-1.25 7-4.4 7-8.6V6.25z"/><path d="m9.25 12.25 2 2 3.5-3.75"/>',
  red: '<path d="M12 18.75h.01"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><path d="M5.25 11.75a10 10 0 0 1 13.5 0"/>',
  reintentar: '<path d="M20.5 12a8.5 8.5 0 1 1-2.9-6.4"/><path d="M20.5 3.5v5.25h-5.25"/>',
  reloj: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  repetir:
    '<path d="M4.5 11V9a2 2 0 0 1 2-2h13"/><path d="m17 4.5 2.5 2.5L17 9.5"/><path d="M19.5 13v2a2 2 0 0 1-2 2h-13"/><path d="m7 19.5-2.5-2.5L7 14.5"/>',
  siguiente: '<path d="M18 5v14"/><path d="M5.5 5.5 15 12l-9.5 6.5z"/>',
  silencio: '<path d="M3.5 9.5h3.5L12 5v14l-5-4.5H3.5z"/><path d="m15.5 9.5 5 5M20.5 9.5l-5 5"/>',
  tuMusica: '<circle cx="12" cy="8.5" r="3.5"/><path d="M5 20.5a7 7 0 0 1 14 0"/>',
  volumen:
    '<path d="M3.5 9.5h3.5L12 5v14l-5-4.5H3.5z"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11"/>'
} as const;
