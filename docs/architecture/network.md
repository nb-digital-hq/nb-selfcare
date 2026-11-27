# Red y consentimiento

> **Estado:** Aplicable.

`src-tauri/src/net.rs` es la única salida. La política combina interruptor general y servicios autorizados.
De fábrica ambos impiden conexiones. Los servicios modelados son `openFoodFacts`, `actualizaciones` y `sincronizacion`. Este último usa la puerta TLS local y no admite URLs en el cliente HTTP genérico.

Open Food Facts recibe un código de barras consultado expresamente; no recibe perfil, diario ni registros de
salud. El servicio de actualizaciones consulta metadatos y descarga artefactos firmados desde fuentes oficiales.
El consentimiento debe describir la petición antes de ejecutarla. Un fallo remoto se devuelve como código y no
impide trabajar localmente.

El guardián estático detecta clientes HTTP fuera de esta puerta. No es una captura exhaustiva de tráfico de las
bibliotecas del sistema; por eso cada publicación se comprueba también con la aplicación real.

## Puerta TLS local

`PuertaLocal` permite enlazar una IP numérica local concreta, con consentimiento general y del servicio. No
admite interfaces comodín. Solo acepta pares locales y conexiones TLS; las negociaciones pendientes se limitan
a 16 y vencen a los cinco segundos. Hay hasta 32 conexiones y cada una dura como máximo 120 segundos. Un cliente que no inicia TLS no bloquea otras negociaciones. Cada petición
comprueba la política antes y después del manejador, con un máximo de diez segundos para ese manejador.
Revocar el servicio o el permiso general detiene la aceptación. El anuncio opcional `_nb-selfcare._tcp.local.`
solo incluye la versión y el puerto, y se retira al detener el servicio. No anuncia nombres personales ni tokens.

Esta infraestructura se prueba con una sonda sintética sobre loopback. No está conectada al arranque ni al
puente de usuario, y su router de pares autentica credenciales revocables. No incorpora aplicación de registros. Ninguna ruta de salud se
expone por disponer de esta puerta. El propietario del router debe añadir autorización, límites de cuerpo y
validación de dominio antes de incorporarlo al producto. El [contrato](sync-contract.md) separa esas garantías.
