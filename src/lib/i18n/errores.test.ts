import { describe, expect, it } from 'vitest';
import { ERROR_CODES } from '#lib/bindings/errorCodes.ts';
import { mensajeError } from './errores';
import { textos, usarIdioma } from './index';

describe('errores visibles sin información privada', () => {
  it('todos los códigos canónicos tienen explicación en los tres idiomas', async () => {
    for (const idioma of ['es', 'en', 'pt']) {
      await usarIdioma(idioma);
      expect(Object.keys(textos.errores).sort()).toEqual([...ERROR_CODES].sort());
      for (const codigo of ERROR_CODES) expect(mensajeError(codigo).length).toBeGreaterThan(10);
    }
    await usarIdioma('es');
  });
  it('un código desconocido no se muestra como mensaje de producto', () => {
    const mensaje = mensajeError('/privado/detalle_medico');
    for (const clave of ['constructor', 'toString', '__proto__'])
      expect(mensajeError(clave)).toBe(textos.errores.INTERNAL);
    expect(mensaje).toBe(textos.errores.INTERNAL);
    expect(mensaje).not.toContain('detalle_medico');
  });
});
