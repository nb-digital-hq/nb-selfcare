import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';

const temporales = [];
afterEach(() => { for (const ruta of temporales.splice(0)) rmSync(ruta, { recursive: true, force: true }); });

function verificar(comando) {
  const raiz = mkdtempSync(join(tmpdir(), 'selfcare-puente-'));
  temporales.push(raiz);
  for (const dir of ['scripts', 'src-tauri/src', 'src/lib/core/ipc', 'src/routes']) mkdirSync(join(raiz, dir), { recursive: true });
  copyFileSync(resolve('scripts/check-puente.mjs'), join(raiz, 'scripts/check-puente.mjs'));
  writeFileSync(join(raiz, 'src-tauri/src/lib.rs'), 'generate_handler![comandos::listar])');
  writeFileSync(join(raiz, 'src/lib/core/ipc/index.ts'), `export function listar(): Promise<Array<Alimento>> {\n return call<Array<Alimento>>('${comando}');\n}\n`);
  writeFileSync(join(raiz, 'src/routes/+page.svelte'), 'listar();');
  return spawnSync(process.execPath, [join(raiz, 'scripts/check-puente.mjs')], { encoding: 'utf8' });
}

describe('guardian del puente con colecciones tipadas', () => {
  it('reconoce comandos dentro de genéricos anidados', () => {
    const resultado = verificar('listar');
    expect(resultado.stderr).toBe('');
    expect(resultado.status).toBe(0);
  });

  it('rechaza un comando inexistente aunque devuelva una colección', () => {
    const resultado = verificar('inexistente');
    expect(resultado.status).toBe(1);
    expect(resultado.stderr).toContain('inexistente');
    expect(resultado.stderr).toContain('no está registrado');
  });
});
