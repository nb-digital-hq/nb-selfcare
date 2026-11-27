import { describe, expect, it } from 'vitest';
import { diaLocal, hora, minutos, numero } from './dia';

describe('fechas y horas locales de registros', () => {
  it('usa el calendario local incluso cerca de medianoche', () => {
    const fecha = new Date(2026, 10, 22, 0, 5);
    expect(diaLocal(fecha)).toBe('2026-11-22');
  });
  it('conserva las horas completas al convertir a minutos', () => {
    for (const m of [0, 1, 59, 60, 480, 1080, 1439]) expect(minutos(hora(m))).toBe(m);
  });
});

it('un cero de coma flotante negativo se muestra como cero', () => {
  expect(numero(-0)).toBe('0');
  expect(numero(-0, 1)).toBe('0');
});
