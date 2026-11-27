/** Acceso único y tipado a los comandos del runtime de NB SelfCare. */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ERROR_CODES } from '#lib/bindings/errorCodes.ts';
import type {
  Actualizacion,
  AjustesAvisos,
  Alimento,
  Apariencia,
  Aviso,
  Bienvenida,
  BorradorTratamiento,
  CambioPerfil,
  Categoria,
  Ciclo,
  Cinta,
  Combo,
  Conflicto,
  DiaComida,
  Entrada,
  EstadoAyuno,
  EstadoEnlace,
  EstadoNibi,
  Evolucion,
  Hoy,
  InformeDeSalud,
  Inspeccion,
  Intensidad,
  IntentoCalibracion,
  InvitacionEnlace,
  Movimiento,
  Noche,
  NuevaEntrada,
  NuevoAlimento,
  NuevoContador,
  PoliticaDeRed,
  Propuesta,
  Receta,
  Recompensa,
  Registrado,
  ResultadoPesaje,
  ResumenCuerpo,
  ResumenCuidado,
  Sugerencia,
  Terminado,
  TipoDeMovimiento
} from '#lib/bindings/index.ts';
import { COMANDOS_QUE_CAMBIAN, cambios } from '../cambios.svelte.ts';

export type * from '#lib/bindings/index.ts';

/** Un fallo sin código nunca revela su contenido, que puede incluir datos privados. */
export class ScError extends Error {
  readonly code: string;
  constructor(code: string) {
    super(code);
    this.name = 'ScError';
    this.code = code;
  }
}

export function aScError(causa: unknown): ScError {
  if (
    typeof causa === 'object' &&
    causa !== null &&
    'code' in causa &&
    typeof causa.code === 'string' &&
    (ERROR_CODES as readonly string[]).includes(causa.code)
  ) {
    return new ScError(causa.code);
  }
  return new ScError('INTERNAL');
}

export async function call<T>(comando: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const resultado = await invoke<T>(comando, args);
    if (COMANDOS_QUE_CAMBIAN.has(comando)) cambios.avisar();
    return resultado;
  } catch (causa) {
    throw aScError(causa);
  }
}

export async function mostrarVentanaPrincipal(): Promise<void> {
  try {
    await getCurrentWindow().show();
  } catch (causa) {
    throw aScError(causa);
  }
}

export function escuchar<T>(evento: string, cada: (carga: T) => void): Promise<UnlistenFn> {
  return listen<T>(evento, (e) => cada(e.payload));
}

export function salud(): Promise<InformeDeSalud> {
  return call<InformeDeSalud>('salud');
}

export function arranqueEnSegundoPlano(): Promise<boolean> {
  return call<boolean>('arranque_en_segundo_plano');
}

export function registrarVentanaNoVisible(): Promise<void> {
  return call<void>('registrar_ventana_no_visible');
}

export function apariencia(): Promise<Apariencia> {
  return call<Apariencia>('apariencia');
}

export function guardarPreferencia(clave: string, valor: unknown): Promise<void> {
  return call<void>('guardar_preferencia', { clave, valor });
}

export function avisosAjustes(): Promise<AjustesAvisos> {
  return call<AjustesAvisos>('avisos_ajustes');
}

export function guardarAvisos(ajustes: AjustesAvisos): Promise<void> {
  return call<void>('guardar_avisos', { ajustes });
}

export function avisosPosponer(minutos: number): Promise<void> {
  return call<void>('avisos_posponer', { minutos });
}

export function politicaRed(): Promise<PoliticaDeRed> {
  return call<PoliticaDeRed>('politica_red');
}

export function cambiarPoliticaRed(politica: PoliticaDeRed): Promise<void> {
  return call<void>('cambiar_politica_red', { politica });
}

export function inicioConSistema(activar: boolean | null): Promise<boolean> {
  return call<boolean>('inicio_con_sistema', { activar });
}

export function actualizacionBuscar(): Promise<Actualizacion | null> {
  return call<Actualizacion | null>('actualizacion_buscar');
}

export function actualizacionInstalar(): Promise<void> {
  return call<void>('actualizacion_instalar');
}

export function hoy(): Promise<Hoy> {
  return call<Hoy>('hoy');
}

export function perfilBienvenida(respuestas: Bienvenida): Promise<void> {
  return call<void>('perfil_bienvenida', { respuestas });
}

export function perfilCambiar(cambio: CambioPerfil): Promise<void> {
  return call<void>('perfil_cambiar', { cambio });
}

export function comidaDia(dia: string): Promise<DiaComida> {
  return call<DiaComida>('comida_dia', { dia });
}

export function comidaBuscar(consulta: string, limit: number): Promise<Array<Alimento>> {
  return call<Array<Alimento>>('comida_buscar', { consulta, limit });
}

export function comidaAlimento(id: number): Promise<Alimento> {
  return call<Alimento>('comida_alimento', { id });
}

export function comidaRegistrar(
  alimento: number,
  gramos: number,
  comida: string
): Promise<Registrado> {
  return call<Registrado>('comida_registrar', { alimento, gramos, comida });
}

export function comidaProponer(frase: string): Promise<Array<Propuesta>> {
  return call<Array<Propuesta>>('comida_proponer', { frase });
}

export function comidaRegistrarFrase(frase: string, comida: string): Promise<Registrado> {
  return call<Registrado>('comida_registrar_frase', { frase, comida });
}

export function comidaRepetirAyer(): Promise<Registrado> {
  return call<Registrado>('comida_repetir_ayer');
}

export function comidaBorrar(ids: Array<number>): Promise<void> {
  return call<void>('comida_borrar', { ids });
}

export function comidaFavorito(id: number, favorito: boolean): Promise<void> {
  return call<void>('comida_favorito', { id, favorito });
}

export function comidaCrear(alimento: NuevoAlimento): Promise<Alimento> {
  return call<Alimento>('comida_crear', { alimento });
}

export function comidaBeber(ml: number): Promise<number> {
  return call<number>('comida_beber', { ml });
}

export function comidaBorrarVaso(id: number): Promise<void> {
  return call<void>('comida_borrar_vaso', { id });
}

export function comidaCombos(): Promise<Array<Combo>> {
  return call<Array<Combo>>('comida_combos');
}

export function comidaGuardarCombo(nombre: string, dia: string, comida: string): Promise<number> {
  return call<number>('comida_guardar_combo', { nombre, dia, comida });
}

export function comidaRegistrarCombo(
  combo: number,
  escala: number,
  comida: string
): Promise<Registrado> {
  return call<Registrado>('comida_registrar_combo', { combo, escala, comida });
}

export function comidaBorrarCombo(id: number): Promise<void> {
  return call<void>('comida_borrar_combo', { id });
}

export function comidaCodigo(codigo: string): Promise<Alimento> {
  return call<Alimento>('comida_codigo', { codigo });
}

export function cuerpoResumen(): Promise<ResumenCuerpo> {
  return call<ResumenCuerpo>('cuerpo_resumen');
}

export function cuerpoPesar(peso: number): Promise<ResultadoPesaje> {
  return call<ResultadoPesaje>('cuerpo_pesar', { peso });
}

export function cuerpoPareceDeOtraPersona(peso: number): Promise<number | null> {
  return call<number | null>('cuerpo_parece_de_otra_persona', { peso });
}

export function cuerpoCinta(cinta: Cinta): Promise<number> {
  return call<number>('cuerpo_cinta', { cinta });
}

export function cuerpoBorrar(id: number): Promise<void> {
  return call<void>('cuerpo_borrar', { id });
}

export function cuerpoEntrenar(
  categoria: Categoria,
  intensidad: Intensidad,
  minutos: number
): Promise<number> {
  return call<number>('cuerpo_entrenar', { categoria, intensidad, minutos });
}

export function cuerpoBorrarEntreno(id: number): Promise<void> {
  return call<void>('cuerpo_borrar_entreno', { id });
}

export function cuerpoCalibrar(): Promise<IntentoCalibracion> {
  return call<IntentoCalibracion>('cuerpo_calibrar');
}

export function ayunoEstado(): Promise<EstadoAyuno> {
  return call<EstadoAyuno>('ayuno_estado');
}

export function ayunoComprobar(horas: number): Promise<Aviso | null> {
  return call<Aviso | null>('ayuno_comprobar', { horas });
}

export function ayunoEmpezar(
  protocolo: string,
  horas: number,
  haceMinutos: number
): Promise<number> {
  return call<number>('ayuno_empezar', { protocolo, horas, haceMinutos });
}

export function ayunoTerminar(): Promise<Terminado> {
  return call<Terminado>('ayuno_terminar');
}

export function ayunoAjustar(haceMinutos: number): Promise<void> {
  return call<void>('ayuno_ajustar', { haceMinutos });
}

export function ayunoBorrar(id: number): Promise<void> {
  return call<void>('ayuno_borrar', { id });
}

export function cuidadoResumen(): Promise<ResumenCuidado> {
  return call<ResumenCuidado>('cuidado_resumen');
}

export function tratamientoGuardar(
  id: number | null,
  tratamiento: BorradorTratamiento
): Promise<number> {
  return call<number>('tratamiento_guardar', { id, tratamiento });
}

export function tratamientoEstado(id: number, activo: boolean | null): Promise<void> {
  return call<void>('tratamiento_estado', { id, activo });
}

export function tratamientoReponer(id: number, unidades: number): Promise<void> {
  return call<void>('tratamiento_reponer', { id, unidades });
}

export function tomaMarcar(
  tratamiento: number,
  hora: number | null,
  tomada: boolean | null
): Promise<void> {
  return call<void>('toma_marcar', { tratamiento, hora, tomada });
}

export function habitoCrear(nombre: string, icono: string, dias: number): Promise<number> {
  return call<number>('habito_crear', { nombre, icono, dias });
}

export function habitoQuitar(id: number, borrar: boolean): Promise<void> {
  return call<void>('habito_quitar', { id, borrar });
}

export function habitoMarcar(id: number, hecho: boolean): Promise<Recompensa | null> {
  return call<Recompensa | null>('habito_marcar', { id, hecho });
}

export function sesionCompletar(
  tipo: string,
  variante: string,
  segundos: number
): Promise<Recompensa | null> {
  return call<Recompensa | null>('sesion_completar', { tipo, variante, segundos });
}

export function vitalMedir(
  tipo: string,
  principal: number,
  secundario: number | null,
  contexto: string | null
): Promise<string> {
  return call<string>('vital_medir', { tipo, principal, secundario, contexto });
}

export function sintomaRegistrar(
  clave: string,
  intensidad: number,
  notas: string | null
): Promise<number> {
  return call<number>('sintoma_registrar', { clave, intensidad, notas });
}

export function chequeoHecho(id: number, dia: string): Promise<Recompensa | null> {
  return call<Recompensa | null>('chequeo_hecho', { id, dia });
}

export function chequeoAjustar(
  id: number | null,
  nombre: string | null,
  meses: number,
  activo: boolean
): Promise<void> {
  return call<void>('chequeo_ajustar', { id, nombre, meses, activo });
}

export function sustanciaConsumir(clave: string, unidades: number): Promise<number> {
  return call<number>('sustancia_consumir', { clave, unidades });
}

export function sustanciaBorrar(id: number): Promise<void> {
  return call<void>('sustancia_borrar', { id });
}

export function libreCrear(contador: NuevoContador): Promise<number> {
  return call<number>('libre_crear', { contador });
}

export function libreGanas(id: number): Promise<Recompensa | null> {
  return call<Recompensa | null>('libre_ganas', { id });
}

export function libreReiniciar(id: number): Promise<void> {
  return call<void>('libre_reiniciar', { id });
}

export function libreCelebrar(id: number): Promise<number | null> {
  return call<number | null>('libre_celebrar', { id });
}

export function libreQuitar(id: number, borrar: boolean): Promise<void> {
  return call<void>('libre_quitar', { id, borrar });
}

export function bienestarGuardar(respuestas: Array<number>): Promise<number> {
  return call<number>('bienestar_guardar', { respuestas });
}

export function diarioEntradas(limit: number): Promise<Array<Entrada>> {
  return call<Array<Entrada>>('diario_entradas', { limit });
}

export function diarioEscribir(entrada: NuevaEntrada): Promise<Recompensa | null> {
  return call<Recompensa | null>('diario_escribir', { entrada });
}

export function diarioBorrar(id: number): Promise<void> {
  return call<void>('diario_borrar', { id });
}

export function diarioSugerir(texto: string): Promise<Sugerencia> {
  return call<Sugerencia>('diario_sugerir', { texto });
}

export function nocheAnotar(noche: Noche): Promise<void> {
  return call<void>('noche_anotar', { noche });
}

export function cafeinaTomar(fuente: string, mg: number): Promise<void> {
  return call<void>('cafeina_tomar', { fuente, mg });
}

export function ciclo(): Promise<Ciclo | null> {
  return call<Ciclo | null>('ciclo');
}

export function cicloEmpezar(dia: string, largo: number): Promise<void> {
  return call<void>('ciclo_empezar', { dia, largo });
}

export function nibiEstado(): Promise<EstadoNibi> {
  return call<EstadoNibi>('nibi_estado');
}

export function nibiComprar(id: string): Promise<void> {
  return call<void>('nibi_comprar', { id });
}

export function nibiPoner(id: string): Promise<void> {
  return call<void>('nibi_poner', { id });
}

export function nibiRenombrar(nombre: string): Promise<void> {
  return call<void>('nibi_renombrar', { nombre });
}

export function copiaExportar(): Promise<boolean> {
  return call<boolean>('copia_exportar');
}

export function copiaElegir(): Promise<Inspeccion | null> {
  return call<Inspeccion | null>('copia_elegir');
}

export function copiaRestaurar(): Promise<Inspeccion> {
  return call<Inspeccion>('copia_restaurar');
}

export function enlaceEstado(): Promise<EstadoEnlace> {
  return call<EstadoEnlace>('enlace_estado');
}

export function enlaceActivar(activar: boolean): Promise<EstadoEnlace> {
  return call<EstadoEnlace>('enlace_activar', { activar });
}

export function enlaceInvitar(): Promise<InvitacionEnlace> {
  return call<InvitacionEnlace>('enlace_invitar');
}

export function enlaceRevocar(nodo: string): Promise<void> {
  return call<void>('enlace_revocar', { nodo });
}

export function sincronizacionConflictos(): Promise<Conflicto[]> {
  return call<Conflicto[]>('sincronizacion_conflictos');
}

export function sincronizacionResolver(
  nodo: string,
  operacion: string,
  usarTelefono: boolean
): Promise<void> {
  return call<void>('sincronizacion_resolver', { nodo, operacion, usarTelefono });
}

export function evolucion(dias: number): Promise<Evolucion> {
  return call<Evolucion>('evolucion', { dias });
}

export function historial(desde: string, hasta: string): Promise<Movimiento[]> {
  return call<Movimiento[]>('historial', { desde, hasta });
}

export function historialBorrar(tipo: TipoDeMovimiento, id: number): Promise<void> {
  return call<void>('historial_borrar', { tipo, id });
}

export function recetas(): Promise<Receta[]> {
  return call<Receta[]>('recetas');
}

export function recetaCrear(
  nombre: string,
  raciones: number,
  dia: string,
  comida: string
): Promise<number> {
  return call<number>('receta_crear', { nombre, raciones, dia, comida });
}

export function recetaRegistrar(id: number, raciones: number, comida: string): Promise<Registrado> {
  return call<Registrado>('receta_registrar', { id, raciones, comida });
}

export function recetaBorrar(id: number): Promise<void> {
  return call<void>('receta_borrar', { id });
}
