import { describe, expect, it } from 'vitest';
import { registrar } from './idioma.svelte';
import { esIdioma, idioma, idiomaDelSistema, localeActual, textos, usarIdioma } from './index';

describe('idioma de SelfCare', () => {
  it('un grupo conservado sigue al idioma actual', async () => {
    const nav = textos.nav;
    await usarIdioma('es');
    expect(nav.ajustes).toBe('Ajustes');
    await usarIdioma('en');
    expect(nav.ajustes).toBe('Settings');
    await usarIdioma('pt');
    expect(nav.ajustes).toBe('Configurações');
    expect(localeActual()).toBe('pt-BR');
    await usarIdioma('es');
  });
  it('los diccionarios diferidos y sus funciones siguen el mismo idioma', async () => {
    const d = registrar(
      { cuenta: (n: number) => `${n} registros` },
      {
        en: async () => ({ cuenta: (n: number) => `${n} records` }),
        pt: async () => ({ cuenta: (n: number) => `${n} registros` })
      }
    );
    await usarIdioma('en');
    expect(d.cuenta(2)).toBe('2 records');
    await usarIdioma('es');
    expect(d.cuenta(2)).toBe('2 registros');
  });
  it('un idioma desconocido usa español', async () => {
    expect(esIdioma('fr')).toBe(false);
    await usarIdioma('fr');
    expect(idioma()).toBe('es');
    expect(textos.nav.hoy).toBe('Hoy');
  });
  it('los catálogos se pueden recorrer como objetos', async () => {
    await usarIdioma('en');
    expect(Object.keys(textos.temas)).toContain('samsung_dark');
    expect(Object.entries(textos.comidas)).toContainEqual(['lunch', 'Lunch']);
    await usarIdioma('es');
  });
  it('reconoce el idioma del sistema o usa uno admitido', () => {
    expect(['es', 'en', 'pt']).toContain(idiomaDelSistema());
  });
  it('los destinos ya importados cambian sus rótulos', async () => {
    const { DESTINOS } = await import('#lib/core/navegacion.ts');
    await usarIdioma('en');
    expect(DESTINOS[0]?.etiqueta).toBe('Today');
    await usarIdioma('es');
    expect(DESTINOS[0]?.etiqueta).toBe('Hoy');
  });
});
