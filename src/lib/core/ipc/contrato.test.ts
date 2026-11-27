import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));

import { aScError, ayunoEmpezar, comidaBuscar, comidaRegistrar, perfilBienvenida } from './index';

beforeEach(() => {
  mocks.invoke.mockReset();
});

describe('contrato real del puente de SelfCare', () => {
  it('envía los nombres camelCase que Tauri espera para argumentos Rust', async () => {
    mocks.invoke.mockResolvedValue(7);
    await expect(ayunoEmpezar('16:8', 16, 30)).resolves.toBe(7);
    expect(mocks.invoke).toHaveBeenCalledWith('ayuno_empezar', {
      protocolo: '16:8',
      horas: 16,
      haceMinutos: 30
    });
  });

  it('conserva los argumentos de búsqueda y registro sin inventar comandos', async () => {
    mocks.invoke.mockResolvedValue([]);
    await comidaBuscar('arroz', 20);
    expect(mocks.invoke).toHaveBeenLastCalledWith('comida_buscar', {
      consulta: 'arroz',
      limit: 20
    });
    await comidaRegistrar(2, 150, 'almuerzo');
    expect(mocks.invoke).toHaveBeenLastCalledWith('comida_registrar', {
      alimento: 2,
      gramos: 150,
      comida: 'almuerzo'
    });
  });

  it('los enteros del perfil se pueden serializar como JSON', async () => {
    mocks.invoke.mockImplementation((_comando, args) => {
      JSON.stringify(args);
      return Promise.resolve();
    });
    await perfilBienvenida({
      nombre: 'Demo',
      nacimiento: '1990-01-01',
      sexo: 'hombre',
      alturaCm: 170,
      pesoActualKg: 70,
      pesoMetaKg: 70,
      ritmoSemanalKg: 0,
      aguaMetaMl: 2000,
      pasosMeta: 6000,
      suenoMetaH: 8,
      antecedentesTca: false,
      tomaHipoglucemiantes: false,
      aparatos: []
    });
    expect(mocks.invoke).toHaveBeenCalledOnce();
  });

  it('solo propaga códigos canónicos y nunca contenido privado', () => {
    expect(aScError({ code: 'STORAGE_FAILURE' }).code).toBe('STORAGE_FAILURE');
    expect(aScError({ code: '/datos/salud/privados' }).code).toBe('INTERNAL');
    expect(aScError(new Error('contenido privado')).message).toBe('INTERNAL');
  });

  it('convierte los rechazos del runtime al error público', async () => {
    mocks.invoke.mockRejectedValue({ code: 'INVALID_INPUT', detail: 'contenido privado' });
    await expect(comidaBuscar('', 0)).rejects.toMatchObject({
      name: 'ScError',
      code: 'INVALID_INPUT'
    });
  });
});
