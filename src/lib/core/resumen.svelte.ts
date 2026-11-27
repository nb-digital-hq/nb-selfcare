import { type Hoy, hoy } from './ipc/index';
import { Recurso } from './recurso.svelte';
export const resumen = new Recurso<Hoy>();
export function actualizarResumen(): Promise<void> {
  return resumen.cargar(hoy);
}
