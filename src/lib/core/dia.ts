import { localeActual } from '#lib/i18n/index.ts';
/** La fecha del registro pertenece al día local, no al UTC. */
export function diaLocal(fecha = new Date()): string {
  return `${fecha.getFullYear()}-${String(fecha.getMonth() + 1).padStart(2, '0')}-${String(fecha.getDate()).padStart(2, '0')}`;
}
export function numero(n: number | null | undefined, decimales = 0): string {
  return n === null || n === undefined || !Number.isFinite(n)
    ? '—'
    : new Intl.NumberFormat(localeActual(), { maximumFractionDigits: decimales }).format(
        n === 0 ? 0 : n
      );
}
export function hora(minutos: number): string {
  return `${String(Math.floor(minutos / 60)).padStart(2, '0')}:${String(minutos % 60).padStart(2, '0')}`;
}
export function minutos(hhmm: string): number {
  const [h = Number.NaN, m = Number.NaN] = hhmm.split(':').map(Number);
  return h * 60 + m;
}
