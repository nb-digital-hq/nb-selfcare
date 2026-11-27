/**
 * Apariencia de NB SelfCare: esquema claro u oscuro, contraste y color de acento.
 *
 * Contrato: docs/adr/decisions/ADR-008-sistema-de-diseno.md
 *
 * La paleta de fábrica sale de `src/paletas.css`, que genera
 * `scripts/generar-paletas.mjs`, para que el primer fotograma ya tenga sus
 * colores. Solo cuando hay un acento propio o un color de portada se calcula la
 * paleta en caliente, y entonces se carga el generador de Material a demanda: el
 * arranque no lo paga.
 */

import type { TemaDelCatalogo } from './temas';

/** Lo que elige el usuario. `sistema` sigue al sistema operativo. */
export type Tema = 'sistema' | 'claro' | 'oscuro';
/** Lo que se pinta de verdad. */
export type Esquema = 'claro' | 'oscuro';
export type Contraste = 'normal' | 'alto';

/** Los que se generan en `paletas.css` (`scripts/generar-paletas.mjs`). */
export const ESQUEMAS: readonly Esquema[] = ['claro', 'oscuro'];

/**
 * Nivel de contraste de Material para cada opción. El alto no cambia la semilla:
 * recalcula cada par de colores contra su fondo con una curva más exigente, que
 * es lo que NB Books midió como el único alto contraste real.
 */
export const NIVEL_DE_CONTRASTE: Readonly<Record<Contraste, number>> = {
  normal: 0,
  alto: 1
};

/** La semilla con la que se generan las paletas de fábrica. */
export const SEMILLA_DE_FABRICA = '#00e5a3';

/** Luminancia relativa de un color `#rrggbb`, de 0 a 1. */
function luminancia(hex: string): number {
  const v = (i: number) => {
    const c = Number.parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.039_28 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * v(1) + 0.7152 * v(3) + 0.0722 * v(5);
}

/** El texto que se lee sobre un color: el más contrastado de los dos extremos. */
function textoSobre(hex: string, claro: string, oscuro: string): string {
  const contraste = (texto: string) => {
    const [mayor, menor] = [luminancia(hex), luminancia(texto)].sort((a, b) => b - a);
    return ((mayor ?? 0) + 0.05) / ((menor ?? 0) + 0.05);
  };
  const propio = contraste(claro) >= contraste(oscuro) ? claro : oscuro;
  if (contraste(propio) >= 4.5) return propio;
  return contraste('#ffffff') >= contraste('#000000') ? '#ffffff' : '#000000';
}

/**
 * Las variables de Material de un tema. Se parte de la paleta que Material
 * genera desde su acento, para que no falte ningún token, y encima se ponen los
 * colores del tema donde cada uno corresponde: así el tema se ve y los componentes
 * que usan tokens que el tema no define siguen siendo coherentes.
 */
export function grupoDeTema(tema: TemaDelCatalogo): 'oscuros' | 'claros' | 'papel' {
  return tema.papel ? 'papel' : luminancia(tema.colores.fondo) < 0.4 ? 'oscuros' : 'claros';
}

export async function variablesDeTema(tema: TemaDelCatalogo): Promise<Record<string, string>> {
  const c = tema.colores;
  const oscuro = luminancia(c.fondo) < 0.4;
  const { generarVariablesM3 } = await import('./generadorM3');
  const base = generarVariablesM3(c.acento, oscuro, 0);
  return {
    ...base,
    '--m3c-surface': c.fondo,
    '--m3c-surface-dim': c.fondo,
    '--m3c-surface-bright': c.superficieAlt,
    '--m3c-surface-container-lowest': c.fondo,
    '--m3c-surface-container-low': c.fondoElevado,
    '--m3c-surface-container': c.superficie,
    '--m3c-surface-container-high': c.superficieAlt,
    '--m3c-surface-container-highest': c.seleccion,
    '--m3c-on-surface': c.texto,
    '--m3c-on-surface-variant': c.textoSec,
    '--m3c-outline': c.textoMuted,
    '--m3c-outline-variant': c.borde,
    '--m3c-primary': c.acento,
    '--m3c-on-primary': textoSobre(c.acento, c.texto, c.fondo),
    '--m3c-primary-container': c.acentoFuerte,
    '--m3c-on-primary-container': textoSobre(c.acentoFuerte, c.texto, c.fondo),
    '--m3c-secondary-container': c.seleccion,
    '--m3c-on-secondary-container': c.texto,
    '--m3c-error': c.peligro,
    '--m3c-on-error': textoSobre(c.peligro, c.texto, c.fondo),
    '--m3c-inverse-surface': c.texto,
    '--m3c-inverse-on-surface': c.fondo,
    '--sc-exito': c.exito,
    '--sc-advertencia': c.advertencia
  };
}

/** El mismo identificador de fábrica que en SelfCare Mobile. */
export const TEMA_DE_FABRICA = 'samsung_dark';

/**
 * Aplica un tema del catálogo al elemento raíz. Devuelve las variables aplicadas,
 * que la carcasa guarda para pintar el siguiente arranque sin esperar al runtime.
 */
export async function aplicarTema(
  raiz: HTMLElement,
  id: string
): Promise<Record<string, string> | null> {
  const { TEMAS } = await import('./temas');
  const tema = TEMAS.find((t) => t.id === id);
  if (!tema) return null;
  raiz.dataset.papel = String(tema.papel);
  const variables = await variablesDeTema(tema);
  aplicarVariables(raiz, variables, luminancia(tema.colores.fondo) < 0.4);
  return variables;
}

/** Escribe unas variables ya calculadas: el primer pintado no espera a nada. */
export function aplicarVariables(
  raiz: HTMLElement,
  variables: Record<string, string>,
  oscuro: boolean
): void {
  raiz.dataset.esquema = oscuro ? 'oscuro' : 'claro';
  for (const nombre of Array.from(raiz.style)) {
    if (
      nombre.startsWith('--m3c-') ||
      nombre.startsWith('--sc-exito') ||
      nombre.startsWith('--sc-advertencia')
    ) {
      raiz.style.removeProperty(nombre);
    }
  }
  for (const [nombre, valor] of Object.entries(variables)) raiz.style.setProperty(nombre, valor);
}
