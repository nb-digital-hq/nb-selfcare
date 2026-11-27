import { describe, expect, it, vi } from 'vitest';
import { Acciones, Recurso } from './recurso.svelte';

vi.mock('./notificaciones.svelte', () => ({ notificar: { exito: vi.fn(), error: vi.fn() } }));
function diferida<T>() {
  let resolver: (valor: T) => void = () => {};
  let rechazar: (causa: unknown) => void = () => {};
  const promesa = new Promise<T>((resolve, reject) => {
    resolver = resolve;
    rechazar = reject;
  });
  return { promesa, resolver, rechazar };
}
describe('lecturas concurrentes de una vista', () => {
  it('una respuesta antigua no reemplaza la búsqueda más reciente', async () => {
    const r = new Recurso<string>();
    const vieja = diferida<string>();
    const nueva = diferida<string>();
    const a = r.cargar(() => vieja.promesa);
    const b = r.cargar(() => nueva.promesa);
    nueva.resolver('nuevo');
    await b;
    vieja.resolver('viejo');
    await a;
    expect(r.datos).toBe('nuevo');
    expect(r.cargando).toBe(false);
  });
  it('un rechazo antiguo no ensucia el resultado reciente', async () => {
    const r = new Recurso<string>();
    const vieja = diferida<string>();
    const a = r.cargar(() => vieja.promesa);
    await r.cargar(async () => 'nuevo');
    vieja.rechazar({ code: 'STORAGE_FAILURE' });
    await a;
    expect(r.datos).toBe('nuevo');
    expect(r.fallo).toBeNull();
  });
  it('desmontar invalida la respuesta pendiente', async () => {
    const r = new Recurso<string>();
    const pendiente = diferida<string>();
    const trabajo = r.cargar(() => pendiente.promesa);
    r.cancelar();
    pendiente.resolver('tarde');
    await trabajo;
    expect(r.datos).toBeNull();
  });
  it('un fallo de lectura no se convierte en una lista vacía válida', async () => {
    const r = new Recurso<string[]>();
    await r.cargar(async () => {
      throw { code: 'STORAGE_FAILURE' };
    });
    expect(r.datos).toBeNull();
    expect(r.fallo).toBe('STORAGE_FAILURE');
  });
});
describe('envío de formularios', () => {
  it('impide una escritura duplicada mientras la primera está pendiente', async () => {
    const acciones = new Acciones();
    const pendiente = diferida<void>();
    const enviar = vi.fn(() => pendiente.promesa);
    const primero = acciones.hacer(enviar);
    await acciones.hacer(enviar);
    expect(enviar).toHaveBeenCalledTimes(1);
    pendiente.resolver();
    await primero;
    expect(acciones.ocupado).toBe(false);
  });
  it('libera el formulario después de fallar y permite reintentar', async () => {
    const acciones = new Acciones();
    await acciones.hacer(async () => {
      throw new Error('privado');
    });
    expect(acciones.ocupado).toBe(false);
    const reintento = vi.fn(async () => {});
    await acciones.hacer(reintento);
    expect(reintento).toHaveBeenCalledOnce();
  });
});
