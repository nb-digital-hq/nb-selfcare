import { textos } from './index';
/** Las claves técnicas nunca se muestran como texto de producto. */
export function nombreCatalogo(clave: string): string {
  const catalogo = textos.catalogo as Readonly<Record<string, string>>;
  const nombre = catalogo[clave];
  return typeof nombre === 'string' ? nombre : textos.ui.tipo;
}
