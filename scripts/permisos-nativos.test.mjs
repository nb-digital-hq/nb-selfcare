import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const raiz = new URL('../src-tauri/', import.meta.url);
describe('recursos nativos del enlace macOS', () => {
  it('declara únicamente el servicio Bonjour propio y explica el consentimiento local', () => {
    const plist = readFileSync(new URL('Info.plist', raiz), 'utf8');
    expect(plist).toContain('<key>NSLocalNetworkUsageDescription</key>');
    expect(plist).toMatch(/<key>NSBonjourServices<\/key>\s*<array>\s*<string>_nb-selfcare\._tcp<\/string>\s*<\/array>/);
  });
  it('empaqueta la explicación del permiso en los tres idiomas en rutas nativas de localización', () => {
    const config = JSON.parse(readFileSync(new URL('tauri.conf.json', raiz), 'utf8'));
    for (const idioma of ['es', 'en', 'pt']) {
      const origen = `locales/${idioma}.lproj/InfoPlist.strings`;
      expect(config.bundle.resources[origen]).toBe(`${idioma}.lproj/InfoPlist.strings`);
      expect(readFileSync(new URL(origen, raiz), 'utf8')).toMatch(/^"NSLocalNetworkUsageDescription" = ".+";\s*$/);
    }
  });
});
