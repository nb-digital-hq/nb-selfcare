/** Inventario de producción desde el grafo instalado, sin depender de la caché de pnpm. */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

export function inventariarNode(raices, leerManifiesto = (carpeta) => JSON.parse(readFileSync(join(carpeta, 'package.json'), 'utf8'))) {
  const paquetes = new Map();
  const visitados = new Set();
  const nodos = new WeakSet();
  function recorrer(nodo) {
    if (!nodo.path) throw new Error('Paquete instalado sin ruta en el inventario');
    if (nodos.has(nodo)) return;
    nodos.add(nodo);
    if (!visitados.has(nodo.path)) {
    visitados.add(nodo.path);
    const manifiesto = leerManifiesto(nodo.path);
    if (typeof manifiesto.name !== 'string' || typeof manifiesto.version !== 'string') {
      throw new Error('El inventario requiere nombre y versión en cada manifiesto instalado');
    }
    const licencia = typeof manifiesto.license === 'string' ? manifiesto.license : manifiesto.license?.type ?? 'SIN DECLARAR';
    const clave = `${manifiesto.name}@${manifiesto.version}`;
    const previo = paquetes.get(clave);
    if (previo && previo.licencia !== licencia) throw new Error(`Licencias contradictorias para ${clave}`);
    if (previo) previo.carpetas.push(nodo.path);
    else paquetes.set(clave, {
      nombre: manifiesto.name,
      // El manifiesto atribuye la versión distribuida, incluso si pnpm normaliza ceros iniciales.
      version: manifiesto.version,
      licencia,
      repositorio: manifiesto.homepage ?? (typeof manifiesto.repository === 'string' ? manifiesto.repository : manifiesto.repository?.url) ?? '',
      carpetas: [nodo.path]
    });
    }
    for (const grupo of ['dependencies', 'optionalDependencies']) {
      for (const hijo of Object.values(nodo[grupo] ?? {})) recorrer(hijo);
    }
  }
  for (const raiz of raices) {
    for (const grupo of ['dependencies', 'optionalDependencies']) {
      for (const nodo of Object.values(raiz[grupo] ?? {})) recorrer(nodo);
    }
  }
  return [...paquetes.values()].sort((a, b) => a.nombre.localeCompare(b.nombre) || a.version.localeCompare(b.version));
}
