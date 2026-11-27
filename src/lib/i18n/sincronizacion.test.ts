import { describe, expect, it } from 'vitest';
import { DICCIONARIOS } from './sincronizacion';

function claves(o: object, prefijo = ''): string[] {
  return Object.entries(o).flatMap(([k, v]) =>
    typeof v === 'string' ? [`${prefijo}${k}`] : claves(v as object, `${prefijo}${k}.`)
  );
}

describe('textos de conexión y evolución', () => {
  it('los tres idiomas tienen las mismas claves y ningún texto vacío', () => {
    const base = claves(DICCIONARIOS.es).sort();
    for (const d of [DICCIONARIOS.en, DICCIONARIOS.pt]) {
      expect(claves(d).sort()).toEqual(base);
    }
    for (const d of Object.values(DICCIONARIOS)) {
      for (const v of Object.values(d).flatMap((g) => Object.values(g))) expect(v).not.toBe('');
    }
  });
  it('conservan los marcadores de sustitución', () => {
    for (const [grupo, textos] of Object.entries(DICCIONARIOS.es)) {
      for (const [k, v] of Object.entries(textos)) {
        const marcas = String(v).match(/\{[a-z]+\}/g) ?? [];
        for (const d of [DICCIONARIOS.en, DICCIONARIOS.pt]) {
          const otro = String((d as Record<string, Record<string, string>>)[grupo]?.[k]);
          for (const m of marcas) expect(otro).toContain(m);
        }
      }
    }
  });
});
