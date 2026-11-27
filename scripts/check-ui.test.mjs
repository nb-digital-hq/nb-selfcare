import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';

const temporales = [];
afterEach(() => { for (const ruta of temporales.splice(0)) rmSync(ruta, { recursive: true, force: true }); });

function verificar(base, componente) {
  const raiz = mkdtempSync(join(tmpdir(), 'selfcare-tokens-'));
  temporales.push(raiz);
  writeFileSync(join(raiz, 'app.css'), base);
  writeFileSync(join(raiz, 'Aviso.svelte'), `<div></div><style>${componente}</style>`);
  return spawnSync(process.execPath, [resolve('scripts/check-ui.mjs'), raiz], { encoding: 'utf8' });
}

describe('tokens propios del sistema de diseño', () => {
  it('rechaza una posición que depende de un token inexistente', () => {
    const resultado = verificar(':root { --sc-espacio-3: 0.75rem; }', '.aviso { bottom: calc(var(--sc-alto-reproductor) + var(--sc-espacio-3)); }');
    expect(resultado.status).toBe(1);
    expect(resultado.stderr).toContain('--sc-alto-reproductor');
  });
  it('admite un token definido por la hoja común', () => {
    expect(verificar(':root { --sc-espacio-3: 0.75rem; }', '.aviso { bottom: var(--sc-espacio-3); }').status).toBe(0);
  });
  it('admite una variable opcional con respaldo explícito', () => {
    expect(verificar('', '.aviso { bottom: var(--sc-opcional, 0); }').status).toBe(0);
  });
});
