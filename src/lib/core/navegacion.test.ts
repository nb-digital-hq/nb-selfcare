import { describe, expect, it } from 'vitest';
import { DESTINOS, estaActivo, rutaDeArranque, rutaGuardadaValida } from './navegacion';

describe('navegación de SelfCare', () => {
  it('expone ocho destinos únicos y Ajustes al final', () => {
    expect(DESTINOS).toHaveLength(8);
    expect(DESTINOS.at(-1)?.id).toBe('ajustes');
    expect(new Set(DESTINOS.map((d) => d.ruta)).size).toBe(8);
    expect(new Set(DESTINOS.map((d) => d.id)).size).toBe(8);
  });
  it('la raíz instalada vacía pertenece a Hoy y las subrutas no se confunden', () => {
    const hoy = DESTINOS[0];
    const comida = DESTINOS[1];
    if (!hoy || !comida) throw new Error('faltan destinos');
    expect(estaActivo(hoy, '')).toBe(true);
    expect(estaActivo(hoy, '/comida')).toBe(false);
    expect(estaActivo(comida, '/comida/detalle')).toBe(true);
    expect(estaActivo(comida, '/comidas')).toBe(false);
  });
  it('solo restaura rutas propias conocidas', () => {
    expect(rutaGuardadaValida('/cuidado')).toBe(true);
    for (const ruta of [
      null,
      '',
      '//ejemplo.com',
      'https://ejemplo.com',
      '/biblioteca',
      '/comida?buscar=privado'
    ])
      expect(rutaGuardadaValida(ruta)).toBe(false);
  });
});

describe('restauración desde la URL inicial del escritorio', () => {
  it('reconoce el pathname vacío de tauri sin perder la última vista', () => {
    expect(rutaDeArranque('', '/nibi')).toBe('/nibi');
    expect(rutaDeArranque('/', '/cuerpo')).toBe('/cuerpo');
  });
  it('no reemplaza una navegación explícita ni abre una dirección desconocida', () => {
    expect(rutaDeArranque('/comida', '/nibi')).toBeNull();
    expect(rutaDeArranque('', 'https://ejemplo.com')).toBeNull();
    expect(rutaDeArranque('', null)).toBeNull();
  });
});
