import { describe, expect, it } from 'vitest';
import { buscarAlimentos, busquedaGlobal } from './busqueda.svelte';

describe('búsqueda compartida en memoria', () => {
  it('ignora consultas vacías y vuelve a emitir la misma consulta explícita', () => {
    const revision = busquedaGlobal.revision;
    expect(buscarAlimentos('   ')).toBe(false);
    expect(busquedaGlobal.revision).toBe(revision);
    expect(buscarAlimentos(' arroz ')).toBe(true);
    expect(busquedaGlobal.consulta).toBe('arroz');
    buscarAlimentos('arroz');
    expect(busquedaGlobal.revision).toBe(revision + 2);
  });
  it('acota el texto antes de enviarlo al catálogo', () => {
    buscarAlimentos('a'.repeat(1000));
    expect(busquedaGlobal.consulta).toHaveLength(120);
  });
});
