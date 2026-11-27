# Sincronización local con SelfCare Mobile

> **Estado:** Aceptado.

## Contexto

El intercambio de copias no mantiene dos instalaciones actualizadas. Cada dispositivo debe conservar el uso
sin conexión y los registros creados mientras el otro no está disponible.

## Decisión

Se incorpora sincronización bidireccional consentida entre escritorio y móvil, con emparejamiento local,
transporte cifrado, certificado fijado, credenciales revocables y almacenamiento seguro del sistema.
No se utiliza una cuenta, un servidor externo ni la identidad de otro producto de la suite.

Los registros reciben identidad estable y revisión base. Los reintentos no duplican operaciones, las bajas se
propagan como operaciones y una edición concurrente no descarta silenciosamente otro cambio. Las reglas de
salud y recompensas se validan en el propietario: sincronizar no autoriza insertar filas arbitrarias ni premiar
una toma. Las proyecciones derivadas no se suman como si fueran registros independientes.

El protocolo propio de SelfCare no modifica los de Sound ni Books. La puerta de red sigue siendo única y el
servicio debe poder detenerse y revocar un dispositivo sin impedir el trabajo local. Linux, Windows y macOS
comparten contrato y criterios de prueba. Las copias siguen siendo una función independiente.

## Consecuencias

Ambas aplicaciones mantienen sus modelos, migraciones, autorizaciones y pruebas en el mismo trabajo. El modelo
de lotes está definido en el [contrato](../architecture/sync-contract.md).

El escritorio actúa como centro de reconciliación: el teléfono captura la mayoría de los registros y el escritorio
aporta revisión semanal, tendencias e historial corregible sobre los datos reunidos. El esquema 9 es común y se
genera desde una sola especificación del dominio.
