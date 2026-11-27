import { textos } from '#lib/i18n/index.ts';
import type { NombreDeIcono } from '#lib/ui/iconos.ts';
export interface Destino {
  readonly id: string;
  readonly ruta: string;
  readonly etiqueta: string;
  readonly icono: NombreDeIcono;
}
export const DESTINOS: readonly Destino[] = [
  {
    id: 'hoy',
    ruta: '/',
    get etiqueta() {
      return textos.nav.hoy;
    },
    icono: 'inicio'
  },
  {
    id: 'comida',
    ruta: '/comida',
    get etiqueta() {
      return textos.nav.comida;
    },
    icono: 'comida'
  },
  {
    id: 'cuerpo',
    ruta: '/cuerpo',
    get etiqueta() {
      return textos.nav.cuerpo;
    },
    icono: 'cuerpo'
  },
  {
    id: 'cuidado',
    ruta: '/cuidado',
    get etiqueta() {
      return textos.nav.cuidado;
    },
    icono: 'cuidado'
  },
  {
    id: 'ayuno',
    ruta: '/ayuno',
    get etiqueta() {
      return textos.nav.ayuno;
    },
    icono: 'reloj'
  },
  {
    id: 'nibi',
    ruta: '/nibi',
    get etiqueta() {
      return textos.nav.nibi;
    },
    icono: 'nibi'
  },
  {
    id: 'perfil',
    ruta: '/perfil',
    get etiqueta() {
      return textos.nav.perfil;
    },
    icono: 'tuMusica'
  },
  {
    id: 'ajustes',
    ruta: '/ajustes',
    get etiqueta() {
      return textos.nav.ajustes;
    },
    icono: 'ajustes'
  }
];
export function estaActivo(destino: Destino, ruta: string): boolean {
  if (destino.ruta === '/') return ruta === '/' || ruta === '';
  return ruta === destino.ruta || ruta.startsWith(`${destino.ruta}/`);
}
/** Solo se restaura una ruta propia conocida; nunca URLs ni rutas arbitrarias. */
export function rutaGuardadaValida(ruta: string | null): ruta is string {
  return DESTINOS.some((d) => d.ruta === ruta);
}

/** Tauri puede abrir con pathname vacío; solo se restaura desde esa raíz inicial. */
export function rutaDeArranque(actual: string, guardada: string | null): string | null {
  return (actual === '' || actual === '/') && rutaGuardadaValida(guardada) && guardada !== '/'
    ? guardada
    : null;
}
