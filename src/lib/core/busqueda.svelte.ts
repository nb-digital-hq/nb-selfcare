/** Búsqueda de alimentos en memoria: no introduce contenido personal en URL ni almacenamiento web. */
export const busquedaGlobal = $state({ consulta: '', revision: 0 });
export function buscarAlimentos(consulta: string): boolean {
  const limpia = consulta.trim().slice(0, 120);
  if (!limpia) return false;
  busquedaGlobal.consulta = limpia;
  busquedaGlobal.revision += 1;
  return true;
}
