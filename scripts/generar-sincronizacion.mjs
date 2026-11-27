#!/usr/bin/env node
/**
 * Resuelve el dominio de sincronización contra el esquema 8 y genera:
 *
 * - `src-tauri/sincronizacion/adaptadores-v1.json`: columnas compartidas, tipos y referencias de cada tabla.
 * - `src-tauri/migrations/V9__sincronizacion.sql`: identidades, revisiones, diario y disparadores.
 *
 * SelfCare Mobile conserva copias idénticas de ambos archivos; sus pruebas comparan los bytes.
 * Con `--comprobar` falla si los archivos versionados no coinciden con la generación.
 *
 * Uso: node scripts/generar-sincronizacion.mjs [--comprobar]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const ROOT = resolve(import.meta.dirname, '..');
const leer = (r) => readFileSync(resolve(ROOT, r), 'utf8');
const dominio = JSON.parse(leer('src-tauri/sincronizacion/dominio-v1.json'));
const esquema = leer('src-tauri/migrations/V1__inicial.sql');

/** Columnas de cada CREATE TABLE del esquema 8, en orden. */
function columnas(tabla) {
  const linea = esquema.split('\n').find((l) => l.startsWith(`CREATE TABLE "${tabla}" (`));
  if (!linea) throw new Error(`tabla desconocida: ${tabla}`);
  const cuerpo = linea.slice(linea.indexOf('(') + 1, linea.lastIndexOf(')'));
  const partes = [];
  let nivel = 0;
  let actual = '';
  let cadena = false;
  for (const c of cuerpo) {
    if (c === "'") cadena = !cadena;
    if (!cadena && c === '(') nivel++;
    if (!cadena && c === ')') nivel--;
    if (c === ',' && nivel === 0 && !cadena) {
      partes.push(actual.trim());
      actual = '';
    } else actual += c;
  }
  partes.push(actual.trim());
  return partes
    .filter((p) => p.startsWith('"'))
    .map((p) => {
      const nombre = p.slice(1, p.indexOf('"', 1));
      const resto = p.slice(nombre.length + 2).trim();
      const sqlTipo = resto.split(' ')[0];
      const booleano = /CHECK \("[a-z0-9_]+" IN \(0, 1\)\)/.test(resto);
      const tipo = booleano
        ? 'booleano'
        : { TEXT: 'texto', REAL: 'real', INTEGER: 'entero', BLOB: 'binario' }[sqlTipo];
      if (!tipo) throw new Error(`tipo no admitido en ${tabla}.${nombre}`);
      return {
        nombre,
        tipo,
        nulo: !resto.includes('NOT NULL'),
        defecto: resto.includes('DEFAULT'),
        clave: resto.includes('PRIMARY KEY')
      };
    });
}

const resueltas = dominio.tablas.map((t) => {
  const todas = columnas(t.tabla);
  const locales = new Set(t.locales ?? []);
  for (const l of locales) {
    if (!todas.some((c) => c.nombre === l)) throw new Error(`local inexistente: ${t.tabla}.${l}`);
  }
  const clave = todas.find((c) => c.clave)?.nombre ?? 'date_key';
  const compartidas = todas.filter((c) => !locales.has(c.nombre) && c.nombre !== 'id');
  for (const c of todas.filter((c) => locales.has(c.nombre))) {
    if (!c.nulo && !c.defecto) throw new Error(`local sin valor por defecto: ${t.tabla}.${c.nombre}`);
  }
  if (compartidas.some((c) => c.tipo === 'binario')) throw new Error(`binario compartido en ${t.tabla}`);
  const referencias = t.referencias ?? {};
  for (const [col, destino] of Object.entries(referencias)) {
    if (!compartidas.some((c) => c.nombre === col)) throw new Error(`referencia sin columna: ${col}`);
    const orden = dominio.tablas.findIndex((x) => x.tabla === destino);
    if (orden < 0 || orden >= dominio.tablas.indexOf(t)) throw new Error(`referencia fuera de orden: ${col}`);
  }
  const natural = (t.unico ? [[]] : (t.natural ?? [])).map((a) =>
    Array.isArray(a) ? { columnas: a, excepto: {} } : { excepto: {}, ...a }
  );
  for (const a of natural) {
    for (const c of a.columnas) {
      if (!compartidas.some((x) => x.nombre === c) && c !== clave) throw new Error(`natural: ${t.tabla}.${c}`);
    }
  }
  return {
    tabla: t.tabla,
    entidad: t.entidad,
    clave,
    unico: Boolean(t.unico),
    publicar_si: t.publicar_si ?? null,
    columnas: compartidas.map(({ nombre, tipo, nulo }) => ({ nombre, tipo, nulo })),
    referencias,
    natural
  };
});

const entidades = new Set();
for (const t of resueltas) {
  if (entidades.has(t.entidad)) throw new Error(`entidad repetida: ${t.entidad}`);
  entidades.add(t.entidad);
}

const adaptadores = `${JSON.stringify({ version: 1, tablas: resueltas }, null, 2)}\n`;

const conTabla = (sql, t, fila) =>
  sql.replaceAll('$T', t.tabla).replaceAll('$E', t.entidad).replaceAll('$K', `CAST(${fila}."${t.clave}" AS TEXT)`);
const publicar = (t, fila) =>
  t.publicar_si ? ` AND ${t.publicar_si.replace(/\b([a-z_]+)\b(?= <>| =)/g, `${fila}.$1`)}` : '';

let sql = `-- Generado por scripts/generar-sincronizacion.mjs. No se edita a mano.
-- Esquema 9 de NB SelfCare, idéntico en escritorio y SelfCare Mobile (ADR-014).
-- No modifica tablas del esquema 8: añade identidades estables, revisiones, diario de
-- cambios, recibos y conflictos. Los disparadores capturan cada escritura en su misma
-- transacción; al aplicar un cambio remoto el propietario marca sc_sync_control.

CREATE TABLE sc_sync_contexto (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    epoca TEXT NOT NULL CHECK (length(epoca) = 32 AND epoca NOT GLOB '*[^0-9a-f]*')
);
INSERT INTO sc_sync_contexto (id, epoca) VALUES (1, lower(hex(randomblob(16))));
CREATE TABLE sc_sync_control (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    aplicando INTEGER NOT NULL DEFAULT 0 CHECK (aplicando IN (0, 1)),
    origen TEXT NULL CHECK (origen IS NULL OR (length(origen) = 32 AND origen NOT GLOB '*[^0-9a-f]*')),
    registro TEXT NULL CHECK (registro IS NULL OR (length(registro) = 32 AND registro NOT GLOB '*[^0-9a-f]*'))
);
INSERT INTO sc_sync_control (id, aplicando, origen, registro) VALUES (1, 0, NULL, NULL);
CREATE TABLE sc_sync_revisiones (
    entidad TEXT NOT NULL CHECK (length(entidad) BETWEEN 1 AND 32),
    registro TEXT NOT NULL CHECK (length(registro) = 32 AND registro NOT GLOB '*[^0-9a-f]*'),
    revision INTEGER NOT NULL CHECK (typeof(revision) = 'integer' AND revision BETWEEN 1 AND 9007199254740991),
    PRIMARY KEY (entidad, registro)
);
CREATE TABLE sc_sync_identidades (
    tabla TEXT NOT NULL,
    clave TEXT NOT NULL,
    registro TEXT NOT NULL UNIQUE CHECK (length(registro) = 32 AND registro NOT GLOB '*[^0-9a-f]*'),
    compartida INTEGER NOT NULL DEFAULT 0 CHECK (compartida BETWEEN 0 AND 9007199254740991),
    eliminado INTEGER NOT NULL DEFAULT 0 CHECK (eliminado IN (0, 1)),
    en_conflicto INTEGER NOT NULL DEFAULT 0 CHECK (en_conflicto IN (0, 1)),
    PRIMARY KEY (tabla, clave)
);
CREATE TABLE sc_sync_diario (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    registro TEXT NOT NULL UNIQUE REFERENCES sc_sync_identidades (registro) ON UPDATE CASCADE,
    origen TEXT NULL
);
CREATE TABLE sc_sync_recibos (
    nodo TEXT NOT NULL CHECK (length(nodo) = 32 AND nodo NOT GLOB '*[^0-9a-f]*'),
    operacion TEXT NOT NULL CHECK (length(operacion) = 32 AND operacion NOT GLOB '*[^0-9a-f]*'),
    epoca TEXT NOT NULL CHECK (length(epoca) = 32 AND epoca NOT GLOB '*[^0-9a-f]*'),
    huella BLOB NOT NULL CHECK (typeof(huella) = 'blob' AND length(huella) = 32),
    resultado TEXT NOT NULL CHECK (resultado IN ('aplicado', 'conflicto')),
    revision INTEGER NOT NULL CHECK (typeof(revision) = 'integer' AND revision BETWEEN 0 AND 9007199254740991),
    PRIMARY KEY (nodo, operacion)
);
CREATE TABLE sc_sync_conflictos (
    nodo TEXT NOT NULL,
    operacion TEXT NOT NULL,
    actual TEXT CHECK (actual IS NULL OR (json_valid(actual) AND json_type(actual) = 'object'
        AND length(CAST(actual AS BLOB)) <= 65536)),
    propuesta TEXT NOT NULL CHECK (json_valid(propuesta) AND json_type(propuesta) = 'object'
        AND length(CAST(propuesta AS BLOB)) <= 69632),
    PRIMARY KEY (nodo, operacion),
    FOREIGN KEY (nodo, operacion) REFERENCES sc_sync_recibos(nodo, operacion) ON DELETE RESTRICT
);
CREATE TABLE sc_sync_pares (
    nodo TEXT NOT NULL PRIMARY KEY CHECK (length(nodo) = 32 AND nodo NOT GLOB '*[^0-9a-f]*'),
    epoca TEXT NULL,
    cursor INTEGER NOT NULL DEFAULT 0 CHECK (cursor >= 0),
    ultima TEXT NULL
);
`;

const aplicando = '(SELECT aplicando FROM sc_sync_control WHERE id = 1)';
const identidad = (t, fila) =>
  `SELECT registro FROM sc_sync_identidades WHERE tabla = '${t.tabla}' AND clave = ${conTabla('$K', t, fila)}`;
const marcar = (t, fila) => `    UPDATE sc_sync_identidades SET en_conflicto = 0
        WHERE tabla = '${t.tabla}' AND clave = ${conTabla('$K', t, fila)} AND ${aplicando} = 0;
    INSERT INTO sc_sync_revisiones (entidad, registro, revision)
        SELECT '${t.entidad}', registro, compartida + 1 FROM sc_sync_identidades
        WHERE tabla = '${t.tabla}' AND clave = ${conTabla('$K', t, fila)} AND ${aplicando} = 0${publicar(t, fila)}
        ON CONFLICT (entidad, registro) DO UPDATE SET revision = excluded.revision;
    DELETE FROM sc_sync_diario WHERE registro = (${identidad(t, fila)});
    INSERT INTO sc_sync_diario (registro, origen)
        SELECT registro, (SELECT origen FROM sc_sync_control WHERE id = 1) FROM sc_sync_identidades
        WHERE tabla = '${t.tabla}' AND clave = ${conTabla('$K', t, fila)}${publicar(t, fila)};
`;

let reconstruir = `-- Generado por scripts/generar-sincronizacion.mjs. No se edita a mano.
-- Asigna identidades nuevas a una base del esquema 8 restaurada o migrada al 9.
`;
for (const t of resueltas) {
  const cols = t.columnas.map((c) => `"${c.nombre}"`).join(', ');
  const relleno = `
-- ${t.tabla} (${t.entidad})
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT '${t.tabla}', CAST("${t.clave}" AS TEXT), lower(hex(randomblob(16))) FROM "${t.tabla}";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT '${t.entidad}', registro, 1 FROM sc_sync_identidades WHERE tabla = '${t.tabla}';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "${t.tabla}" f JOIN sc_sync_identidades i
    ON i.tabla = '${t.tabla}' AND i.clave = CAST(f."${t.clave}" AS TEXT)${t.publicar_si ? ` WHERE f.${t.publicar_si}` : ''};
`;
  reconstruir += relleno;
  sql += `${relleno}CREATE TRIGGER sc_sync_${t.tabla}_ai AFTER INSERT ON "${t.tabla}" BEGIN
    INSERT INTO sc_sync_identidades (tabla, clave, registro)
        VALUES ('${t.tabla}', ${conTabla('$K', t, 'NEW')},
            coalesce((SELECT registro FROM sc_sync_control WHERE id = 1), lower(hex(randomblob(16)))))
        ON CONFLICT (tabla, clave) DO UPDATE SET eliminado = 0;
${marcar(t, 'NEW')}END;
CREATE TRIGGER sc_sync_${t.tabla}_au AFTER UPDATE OF ${cols} ON "${t.tabla}" BEGIN
${marcar(t, 'NEW')}END;
CREATE TRIGGER sc_sync_${t.tabla}_ad AFTER DELETE ON "${t.tabla}" BEGIN
    UPDATE sc_sync_identidades SET eliminado = 1 WHERE tabla = '${t.tabla}' AND clave = ${conTabla('$K', t, 'OLD')};
${marcar(t, 'OLD')}END;
`;
}

const salidas = [
  ['src-tauri/sincronizacion/adaptadores-v1.json', adaptadores],
  ['src-tauri/migrations/V9__sincronizacion.sql', sql],
  ['src-tauri/sincronizacion/reconstruir-v1.sql', reconstruir]
];
// SelfCare Mobile recibe los mismos bytes como constantes Dart.
const dart = `// Generado por scripts/generar-sincronizacion.mjs de NB SelfCare. No se edita a mano.
// ignore_for_file: lines_longer_than_80_chars

/// Esquema 9 compartido con el escritorio (ADR-025).
const String esquemaSincronizacionV9 = r'''
${sql}''';

/// Identidades nuevas para una base del esquema 8 restaurada.
const String reconstruirSincronizacionV1 = r'''
${reconstruir}''';

/// Columnas, referencias y claves naturales de cada tabla compartida.
const String adaptadoresSincronizacionV1 = r'''
${adaptadores}''';
`;
const movil = process.argv.indexOf('--movil');
if (movil > 0) salidas.push([resolve(process.argv[movil + 1], 'lib/core/sync/esquema_v9.dart'), dart]);

if (process.argv.includes('--comprobar')) {
  const distintos = salidas.filter(([r, c]) => {
    try {
      return leer(r) !== c;
    } catch {
      return true;
    }
  });
  if (distintos.length) {
    console.error(`Sincronización desactualizada: ${distintos.map(([r]) => r).join(', ')}`);
    process.exit(1);
  }
  console.log('Sincronización: adaptadores y esquema 9 al día.');
} else {
  for (const [r, c] of salidas) writeFileSync(resolve(ROOT, r), c);
  console.log(`Generados ${resueltas.length} adaptadores y el esquema 9.`);
}
