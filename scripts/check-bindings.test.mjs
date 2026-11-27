import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';

const temporales = [];
afterEach(() => { for (const ruta of temporales.splice(0)) rmSync(ruta, { recursive: true, force: true }); });

// El ejecutable de prueba usa el mismo contrato de proceso que Cargo, sin
// compilar Tauri ni necesitar un repositorio Git en la carpeta temporal.
describe.skipIf(process.platform === 'win32')('deriva de bindings sin depender del estado Git', () => {
  function preparar() {
    const raiz = mkdtempSync(join(tmpdir(), 'selfcare-bindings-'));
    temporales.push(raiz);
    for (const dir of ['scripts', 'src/lib/bindings', 'src-tauri', 'bin']) mkdirSync(join(raiz, dir), { recursive: true });
    copyFileSync(resolve('scripts/check-bindings.mjs'), join(raiz, 'scripts/check-bindings.mjs'));
    const cargo = join(raiz, 'bin/cargo');
    writeFileSync(cargo, '#!/bin/sh\nprintf "export type Dato = number;\\n" > ../src/lib/bindings/Dato.ts\n');
    chmodSync(cargo, 0o755);
    return raiz;
  }
  function ejecutar(raiz) {
    return spawnSync(process.execPath, [join(raiz, 'scripts/check-bindings.mjs')], {
      encoding: 'utf8', env: { ...process.env, PATH: `${join(raiz, 'bin')}${delimiter}${process.env.PATH}` }
    });
  }

  it('rechaza una regeneración que cambia archivos aunque no haya primer commit', () => {
    const raiz = preparar();
    writeFileSync(join(raiz, 'src/lib/bindings/Dato.ts'), 'export type Dato = bigint;\n');
    const resultado = ejecutar(raiz);
    expect(resultado.status).toBe(1);
    expect(resultado.stderr).toContain('Dato.ts');
    expect(readFileSync(join(raiz, 'src/lib/bindings/Dato.ts'), 'utf8')).toContain('number');
  });

  it('acepta contenido estable aunque los archivos no estén versionados', () => {
    const raiz = preparar();
    ejecutar(raiz);
    const resultado = ejecutar(raiz);
    expect(resultado.status).toBe(0);
    expect(resultado.stdout).toContain('al día');
  });
});
