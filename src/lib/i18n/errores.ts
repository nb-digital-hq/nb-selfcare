import type { ErrorCode } from '#lib/bindings/errorCodes.ts';
import { textos } from './index';
export function mensajeError(codigo: string): string {
  const mensajes: Readonly<Record<ErrorCode, string>> = textos.errores;
  const mensaje = mensajes[codigo as ErrorCode];
  return typeof mensaje === 'string' ? mensaje : mensajes.INTERNAL;
}
