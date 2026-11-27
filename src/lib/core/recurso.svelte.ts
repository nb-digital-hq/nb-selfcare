import { mensajeError } from '#lib/i18n/errores.ts';
import { textos } from '#lib/i18n/index.ts';
import { aScError } from './ipc/index';
import { notificar } from './notificaciones.svelte';

/** Solo la petición más reciente puede actualizar una vista aún montada. */
export class Recurso<T> {
  datos = $state.raw<T | null>(null);
  cargando = $state(false);
  actualizadoEn = $state(0);
  fallo = $state<string | null>(null);
  #pedido = 0;
  async cargar(lectura: () => Promise<T>): Promise<void> {
    const pedido = ++this.#pedido;
    this.cargando = true;
    this.fallo = null;
    try {
      const datos = await lectura();
      if (pedido === this.#pedido) {
        this.datos = datos;
        this.actualizadoEn = Date.now();
      }
    } catch (causa) {
      if (pedido === this.#pedido) this.fallo = aScError(causa).code;
    } finally {
      if (pedido === this.#pedido) this.cargando = false;
    }
  }
  cancelar(): void {
    this.#pedido += 1;
  }
}

/** Impide dobles envíos y conserva el formulario cuando falla la escritura. */
export class Acciones {
  ocupado = $state(false);
  async hacer(trabajo: () => Promise<unknown>, anunciar = true): Promise<void> {
    if (this.ocupado) return;
    this.ocupado = true;
    try {
      await trabajo();
      if (anunciar) notificar.exito(textos.comun.guardado);
    } catch (causa) {
      notificar.error(mensajeError(aScError(causa).code));
    } finally {
      this.ocupado = false;
    }
  }
}
