import { describe, expect, it } from 'vitest';
import { grupoDeTema, TEMA_DE_FABRICA, variablesDeTema } from './tema';
import { TEMAS } from './temas';

describe('catálogo de temas de SelfCare', () => {
  it('conserva los 24 identificadores únicos y el tema inicial del móvil', () => {
    expect(TEMAS).toHaveLength(24);
    expect(new Set(TEMAS.map((t) => t.id)).size).toBe(24);
    expect(TEMAS.some((t) => t.id === TEMA_DE_FABRICA)).toBe(true);
    expect(TEMA_DE_FABRICA).toBe('samsung_dark');
  });
  it('cada tema genera todos los tokens y conserva sus colores propios', async () => {
    for (const tema of TEMAS) {
      const v = await variablesDeTema(tema);
      expect(v['--m3c-surface']).toBe(tema.colores.fondo);
      expect(v['--m3c-primary']).toBe(tema.colores.acento);
      expect(v['--m3c-on-surface']).toBe(tema.colores.texto);
      expect(v['--m3c-tertiary']).toMatch(/^#/);
    }
  });
  it('los cuatro temas de papel declaran su tratamiento sin movimiento', () => {
    expect(
      TEMAS.filter((t) => t.papel)
        .map((t) => t.id)
        .sort()
    ).toEqual([
      'nxtpaper_color_paper',
      'nxtpaper_eink_dark',
      'nxtpaper_eink_light',
      'nxtpaper_sepia'
    ]);
  });
});

describe('legibilidad de botones en todos los temas', () => {
  function luminancia(hex: string) {
    const canales = [1, 3, 5].map((i) => {
      const c = Number.parseInt(hex.slice(i, i + 2), 16) / 255;
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return (canales[0] ?? 0) * 0.2126 + (canales[1] ?? 0) * 0.7152 + (canales[2] ?? 0) * 0.0722;
  }
  function contraste(a: string, b: string) {
    const x = luminancia(a),
      y = luminancia(b);
    return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
  }
  it('los textos sobre acento, contenedor y error alcanzan contraste AA', async () => {
    for (const tema of TEMAS) {
      const v = await variablesDeTema(tema);
      for (const rol of ['primary', 'primary-container', 'error']) {
        const fondo = v[`--m3c-${rol}`];
        const texto = v[`--m3c-on-${rol}`];
        if (!fondo || !texto) throw new Error('token ausente');
        expect(contraste(fondo, texto), `${tema.id}: ${rol}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
});

describe('grupos de las muestras de apariencia', () => {
  it('cada tema pertenece a un único grupo y papel conserva sus cuatro temas', () => {
    expect(TEMAS.filter((t) => grupoDeTema(t) === 'papel')).toHaveLength(4);
    expect(TEMAS.filter((t) => grupoDeTema(t) === 'claros').length).toBeGreaterThan(0);
    expect(TEMAS.filter((t) => grupoDeTema(t) === 'oscuros').length).toBeGreaterThan(0);
    expect(TEMAS.every((t) => ['papel', 'claros', 'oscuros'].includes(grupoDeTema(t)))).toBe(true);
  });
});
