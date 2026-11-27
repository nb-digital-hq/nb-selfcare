#!/usr/bin/env node
/**
 * Genera THIRD-PARTY-NOTICES.md (portado de NB Books).
 *
 * Casi todas las licencias de nuestras dependencias obligan a conservar el aviso
 * de copyright y el texto de la licencia al redistribuir, y la licencia de NB SelfCare
 * no exime de eso.
 *
 * Se genera automáticamente: un archivo mantenido a mano queda obsoleto en la
 * primera actualización de dependencias.
 *
 * Uso: node scripts/generate-notices.mjs [--check] [--cdx <archivo>]
 *   --check  falla si el archivo generado difiere del versionado
 *   --cdx    escribe además el listado de componentes en CycloneDX 1.5, que
 *            viaja en cada publicación (docs/developer/releases.md)
 *
 * Ver docs/adr/010-licensing-model.md
 */

import { licenciaAdmitida } from './licencias.mjs';
import { inventariarNode } from './inventario-node.mjs';
import { createHash } from 'node:crypto';
import { dirname } from 'node:path';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const ROOT = resolve(import.meta.dirname, '..');
const SALIDA = join(ROOT, 'THIRD-PARTY-NOTICES.md');

/**
 * Licencias admitidas. Debe coincidir con `deny.toml` y con la política de
 * dependencias, y que coincida **no depende de que nadie se acuerde**:
 * `comprobarListasAlineadas` lo verifica más abajo. Tres listas que dicen lo
 * mismo en tres sitios distintos divergen solas, y la que se queda corta deja
 * pasar una licencia que la política prohíbe.
 */
const PERMITIDAS = new Set([
  'Apache-2.0',
  'Apache-2.0 WITH LLVM-exception',
  'MIT',
  'MIT-0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'ISC',
  'Zlib',
  'Unicode-3.0',
  'CC0-1.0',
  'Unlicense',
  'BSL-1.0',
  'MPL-2.0',
  '0BSD',
  'CDLA-Permissive-2.0',
  // Tipografías: las cinco de static/fuentes/LICENCIAS.md.
  'OFL-1.1'
]);

/**
 * Comprueba que `deny.toml` admite exactamente las mismas licencias que este
 * archivo.
 *
 * Existe porque la política obliga a tocar tres sitios al ampliar la lista, y
 * una regla que exige acordarse de tres sitios se incumple al segundo cambio.
 * Ver deny.toml.
 */
function comprobarListasAlineadas() {
  const deny = readFileSync(join(ROOT, 'deny.toml'), 'utf8');
  const bloque = deny.slice(deny.indexOf('allow = ['), deny.indexOf(']', deny.indexOf('allow = [')));
  const enDeny = new Set([...bloque.matchAll(/"([^"]+)"/g)].map((m) => m[1]));

  const soloAqui = [...PERMITIDAS].filter((l) => !enDeny.has(l));
  const soloEnDeny = [...enDeny].filter((l) => !PERMITIDAS.has(l));

  if (soloAqui.length > 0 || soloEnDeny.length > 0) {
    console.error('Las listas de licencias admitidas no coinciden.\n');
    for (const l of soloAqui) console.error(`  solo en generate-notices.mjs: ${l}`);
    for (const l of soloEnDeny) console.error(`  solo en deny.toml: ${l}`);
    console.error('\nVer deny.toml');
    process.exit(1);
  }
}


function ejecutar(cmd, args, cwd) {
  // En Windows pnpm es un lanzador .cmd que solo se resuelve a través del intérprete de órdenes; los argumentos son
  // constantes del propio script, nunca datos externos.
  return execFileSync(cmd, args, {
    cwd,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    shell: process.platform === 'win32'
  });
}

/** Dependencias de Rust que acaban en el binario. */
function dependenciasRust() {
  const salida = ejecutar('cargo', ['metadata', '--format-version', '1'], join(ROOT, 'src-tauri'));
  const { packages, resolve: grafo, workspace_members: miembros } = JSON.parse(salida);
  const propios = new Set(miembros);

  return grafo.nodes
    .filter((n) => !propios.has(n.id))
    .map((n) => packages.find((p) => p.id === n.id))
    .filter(Boolean)
    .map((p) => ({
      nombre: p.name,
      version: p.version,
      licencia: p.license ?? (p.license_file ? 'ver archivo de licencia' : 'SIN DECLARAR'),
      repositorio: p.repository ?? '',
      carpetas: [dirname(p.manifest_path)]
    }))
    .sort((a, b) => a.nombre.localeCompare(b.nombre));
}

/** Dependencias de Node incluidas en el bundle del frontend. */
function dependenciasNode() {
  const salida = ejecutar('pnpm', ['list', '--json', '--prod', '--depth', 'Infinity'], ROOT);
  return inventariarNode(JSON.parse(salida));
}

/**
 * Código de terceros **copiado dentro** del repositorio, no traído por un gestor
 * de paquetes.
 *
 * `cargo metadata` no lo ve, así que sin esta lista la atribución que exigen MIT
 * y Apache-2.0 se quedaría fuera del archivo que se distribuye. Cada entrada
 * apunta a la carpeta donde vive su `PROCEDENCIA.md`, y más abajo se comprueba
 * que no haya ninguna carpeta vendorizada sin entrada: una lista a mano que nadie
 * comprueba se queda corta en la segunda vez.
 */
const VENDORIZADOS = [];

/** Carpetas con un `PROCEDENCIA.md`: ahí vive código de fuera. */
function carpetasVendorizadas(raiz) {
  const encontradas = [];
  const recorrer = (dir) => {
    for (const entrada of readdirSync(dir)) {
      if (entrada === 'node_modules' || entrada === 'target' || entrada.startsWith('.')) continue;
      const ruta = join(dir, entrada);
      if (statSync(ruta).isDirectory()) recorrer(ruta);
      else if (entrada === 'PROCEDENCIA.md') encontradas.push(relative(raiz, dir));
    }
  };
  recorrer(join(raiz, 'src-tauri', 'src'));
  return encontradas;
}

function comprobarVendorizados(problemas) {
  const declaradas = new Set(VENDORIZADOS.map((v) => v.carpeta));
  for (const carpeta of carpetasVendorizadas(ROOT)) {
    if (!declaradas.has(carpeta)) {
      problemas.push(
        `Vendorizado: ${carpeta} trae código de fuera y no está en VENDORIZADOS de este script`
      );
    }
  }
}

function tablaVendorizados() {
  const filas = VENDORIZADOS.map(
    (v) =>
      `| \`${v.nombre}\` | ${v.version} | ${v.licencia} | ${v.repositorio} | ${v.copyright} | \`${v.carpeta}\` |`
  );
  return [
    '| Component | Version | Licence | Source | Copyright | Dónde vive aquí |',
    '|---|---|---|---|---|---|',
    ...filas
  ].join('\n');
}

function tabla(paquetes) {
  const filas = paquetes.map(
    (p) => `| \`${p.nombre}\` | ${p.version} | ${p.licencia} | ${p.repositorio} |`
  );
  return ['| Component | Version | Licence | Source |', '|---|---|---|---|', ...filas].join('\n');
}

function verificarLicencias(paquetes, origen, problemas) {
  for (const p of paquetes) {
    if (!licenciaAdmitida(p.licencia, PERMITIDAS)) {
      problemas.push(`${origen}: ${p.nombre}@${p.version} declara "${p.licencia}"`);
    }
  }
}

const rust = dependenciasRust();
const node = dependenciasNode();

const problemas = [];
verificarLicencias(rust, 'Rust', problemas);
verificarLicencias(node, 'Node', problemas);
verificarLicencias(VENDORIZADOS, 'Vendorizado', problemas);
comprobarVendorizados(problemas);

/** Conserva los textos y avisos que realmente distribuye cada paquete. */
function textosLegales(paquetes) {
  const textos = new Map();
  for (const paquete of paquetes) {
    let encontrados = 0;
    for (const carpeta of paquete.carpetas ?? []) {
      const archivos = readdirSync(carpeta, { withFileTypes: true }).flatMap((entrada) => {
        const ruta = join(carpeta, entrada.name);
        if (entrada.isFile() && /^(?:LICEN[CS]E|COPYING|NOTICE|COPYRIGHT|UNLICENSE)(?:$|[._-])/i.test(entrada.name)) return [ruta];
        if (entrada.isDirectory() && /^(?:licenses?|licences?)$/i.test(entrada.name)) {
          return readdirSync(ruta, {withFileTypes:true}).filter(e=>e.isFile()).map(e=>join(ruta,e.name));
        }
        return [];
      });
      for (const archivo of archivos.sort()) {
        const contenido = readFileSync(archivo, 'utf8').replaceAll('\r\n','\n').trim();
        if (!contenido) continue;
        encontrados++;
        const hash = createHash('sha256').update(contenido).digest('hex');
        if(!textos.has(hash)) textos.set(hash, {contenido, paquetes:new Set()});
        textos.get(hash).paquetes.add(`${paquete.nombre}@${paquete.version}`);
      }
    }
    if (!encontrados) {
      // Algunos paquetes publican la declaración solo en el manifiesto o README.
      const partes = [`Package: ${paquete.nombre}@${paquete.version}\nDeclared licence: ${paquete.licencia}\nSource: ${paquete.repositorio}\nNo standalone licence file was supplied by this package.`];
      for(const carpeta of paquete.carpetas ?? []) {
        const readme = readdirSync(carpeta).find(nombre => /^readme(?:\.md|\.txt)?$/i.test(nombre));
        if(readme) partes.push(readFileSync(join(carpeta,readme),'utf8'));
      }
      const contenido=partes.join('\n\n').replaceAll('\r\n','\n').trim();
      const hash=createHash('sha256').update(contenido).digest('hex');
      textos.set(hash,{contenido,paquetes:new Set([`${paquete.nombre}@${paquete.version}`])});
    }
  }
  return [...textos.values()].map((texto,i)=>{
    const cerca='`'.repeat(Math.max(3,...[...texto.contenido.matchAll(/`+/g)].map(m=>m[0].length+1)));
    return `### License text ${i+1}\n\nPackages: ${[...texto.paquetes].sort().join(', ')}\n\n${cerca}text\n${texto.contenido}\n${cerca}`;
  }).join('\n\n');
}
const legales = textosLegales([...rust,...node]);

const contenido = `# Third-party notices

<!-- GENERADO AUTOMÁTICAMENTE por scripts/generate-notices.mjs. No editar a mano. -->

NB SelfCare is licensed under Apache-2.0. This file inventories the resolved third-party dependency tree,
including build-time and platform-specific components, and preserves the licence and notice files supplied
by those packages. When a package supplies no standalone licence, its declaration and available README are retained.

Regenerate: \`node scripts/generate-notices.mjs\`

## Included resources

The five local font families retain their OFL-1.1 licences in
\`static/fuentes/LICENCIAS.md\`. The application includes no external executables or inference models.
Self-care rules and identity assets are reused from NB SelfCare Mobile under Apache-2.0 for code and
the separate product trademark policy for brand assets.

## Rust components (${rust.length})

${tabla(rust)}

## JavaScript components (${node.length})

${tabla(node)}

## Original licence and notice texts

${legales}
`;

/**
 * Comprueba que `LICENSE`, `NOTICE` y `THIRD-PARTY-NOTICES.md` viajan dentro del
 * paquete (`bundle.resources` de `src-tauri/tauri.conf.json`) y existen en disco
 *.
 *
 * Ver docs/legal/licensing.md.
 */
function comprobarEmpaquetado() {
  const rutaConfig = join(ROOT, 'src-tauri', 'tauri.conf.json');
  const config = JSON.parse(readFileSync(rutaConfig, 'utf8'));
  const recursos = config?.bundle?.resources ?? {};
  const requeridos = ['../LICENSE', '../NOTICE', '../THIRD-PARTY-NOTICES.md'];
  const faltan = [];

  for (const rel of requeridos) {
    if (!(rel in recursos)) {
      faltan.push(`falta "${rel}" en bundle.resources de src-tauri/tauri.conf.json`);
      continue;
    }
    const rutaReal = resolve(join(ROOT, 'src-tauri'), rel);
    try {
      if (!statSync(rutaReal).isFile()) {
        faltan.push(`"${rel}" no es un archivo regular (${rutaReal})`);
      }
    } catch {
      faltan.push(`"${rel}" está en bundle.resources pero no existe en ${rutaReal}`);
    }
  }

  if (faltan.length > 0) {
    console.error('Los avisos legales no viajan completos en el paquete:\n');
    for (const f of faltan) console.error(`  ${f}`);
    console.error('\nVer docs/adr/010-licensing-model.md');
    process.exit(1);
  }
}

comprobarListasAlineadas();
comprobarEmpaquetado();

/**
 * Una licencia como la quiere CycloneDX: expresión SPDX si lo es, nombre si no.
 * `cargo metadata` aún trae la forma antigua con barra (`MIT/Apache-2.0`), que
 * significa "o" y no es SPDX válido; lo que confiesa que no se declaró licencia
 * va como nombre, porque presentarlo como expresión sería inventarla.
 */
function licenciaCdx(licencia) {
  if (/^(ver |SIN )/.test(licencia)) return [{ license: { name: licencia } }];
  return [{ expression: licencia.replaceAll('/', ' OR ') }];
}

/**
 * El listado de componentes. Sin marca de tiempo propia ni número de serie
 * aleatorio: lleva la fecha del commit, y dos ejecuciones sobre el mismo commit
 * dan el mismo archivo. Un listado que cambia cada vez que se genera no se puede
 * comparar con nada.
 */
function listadoCdx() {
  const git = (...a) => ejecutar('git', a, ROOT).trim();
  const { version } = JSON.parse(readFileSync(join(ROOT, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  const componente = (tipo, p) => ({
    type: 'library',
    name: p.nombre,
    version: p.version,
    purl: `pkg:${tipo}/${p.nombre.replace(/^@/, '%40')}@${p.version}`,
    licenses: licenciaCdx(p.licencia)
  });
  return {
    bomFormat: 'CycloneDX',
    specVersion: '1.5',
    version: 1,
    metadata: {
      timestamp: git('log', '-1', '--format=%cI'),
      component: { type: 'application', name: 'nb-selfcare', version, licenses: [{ expression: 'Apache-2.0' }] },
      properties: [{ name: 'nb-selfcare:commit', value: git('rev-parse', 'HEAD') }]
    },
    components: [
      ...rust.map((p) => componente('cargo', p)),
      // pnpm junta en una fila las versiones de un mismo paquete; aquí cada una
      // es un componente, que es lo que son.
      ...node.flatMap((p) => p.version.split(', ').map((v) => componente('npm', { ...p, version: v })))
    ]
  };
}

const iCdx = process.argv.indexOf('--cdx');
if (iCdx > 0) {
  writeFileSync(process.argv[iCdx + 1], `${JSON.stringify(listadoCdx(), null, 2)}\n`);
  console.log(`Listado CycloneDX: ${process.argv[iCdx + 1]}`);
}

if (process.argv.includes('--check')) {
  let versionado = '';
  try {
    versionado = readFileSync(SALIDA, 'utf8');
  } catch {
    console.error('THIRD-PARTY-NOTICES.md no existe. Ejecuta: node scripts/generate-notices.mjs');
    process.exit(1);
  }
  if (versionado !== contenido) {
    console.error('THIRD-PARTY-NOTICES.md está desactualizado. Ejecuta: node scripts/generate-notices.mjs');
    process.exit(1);
  }
  console.log('Avisos de terceros: al día.');
} else {
  writeFileSync(SALIDA, contenido);
  console.log(`Avisos generados: ${rust.length} componentes de Rust, ${node.length} de JavaScript.`);
}

if (problemas.length > 0) {
  console.error('\nLicencias no permitidas por la política de dependencias:\n');
  for (const p of problemas) console.error(`  ${p}`);
  console.error('\nVer docs/architecture/dependency-policy.md');
  process.exit(1);
}
