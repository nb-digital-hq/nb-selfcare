import { describe, expect, it } from 'vitest';
import { inventariarNode } from './inventario-node.mjs';
describe('inventario instalado de producción', () => {
  it('conserva la versión y licencia reales aunque el gestor normalice la versión', () => {
    const resultado = inventariarNode([{ dependencies: { color: { path: '/color', version: '0.4.177' } } }], () => ({ name: 'color', version: '0.4.0177', license: 'Apache-2.0' }));
    expect(resultado[0].version).toBe('0.4.0177');
    expect(resultado[0].licencia).toBe('Apache-2.0');
  });
  it('recorre transitivas y opcionales sin incluir herramientas de desarrollo ni duplicar rutas', () => {
    const compartida = { path: '/compartida' };
    const raiz = { dependencies: { uno: { path: '/uno', dependencies: { compartida } }, dos: { path: '/dos', optionalDependencies: { compartida } } }, devDependencies: { prueba: { path: '/prueba' } } };
    const resultado = inventariarNode([raiz], (path) => ({ name: path.slice(1), version: '1.0.0', license: 'MIT' }));
    expect(resultado.map((p) => p.nombre)).toEqual(['compartida', 'dos', 'uno']);
    expect(resultado[0].carpetas).toEqual(['/compartida']);
  });
  it('una ruta repetida con subárbol ampliado conserva todas sus transitivas', () => {
    const resultado = inventariarNode([{ dependencies: { hoja: { path: '/igual' }, ampliado: { path: '/igual', dependencies: { hijo: { path: '/hijo' } } } } }], (path) => ({ name: path.slice(1), version: '1', license: 'MIT' }));
    expect(resultado.map((p) => p.nombre)).toEqual(['hijo', 'igual']);
    expect(resultado[1].carpetas).toEqual(['/igual']);
  });
  it('una licencia ausente o contradictoria no se convierte en una licencia permitida', () => {
    expect(inventariarNode([{ dependencies: { sin: { path: '/sin' } } }], () => ({ name: 'sin', version: '1' }))[0].licencia).toBe('SIN DECLARAR');
    expect(() => inventariarNode([{ dependencies: { uno: { path: '/uno' }, dos: { path: '/dos' } } }], (path) => ({ name: 'igual', version: '1', license: path === '/uno' ? 'MIT' : 'GPL-3.0' }))).toThrow('Licencias contradictorias');
  });
});
