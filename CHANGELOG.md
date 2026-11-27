# Cambios de NB SelfCare

## 1.0.0 — 2026-11-27

Primera versión de NB SelfCare.

* Ocho destinos: Hoy, Comida, Cuerpo, Cuidado, Ayuno, Nibi, Perfil y Ajustes, con barra lateral, búsqueda de
  alimentos y registro rápido desde cualquier vista (Ctrl + K): comida escrita con tus palabras, agua o peso.
* Hoy: disposición del día, energía y objetivos, lo registrado hoy en los dos equipos, la semana de un vistazo con
  los últimos 30 días, diario, descanso y cafeína.
* Comida: catálogo sin conexión, nutrientes y minerales, analizador determinista en español, inglés y portugués,
  combinaciones, recetas por raciones, agua y código de barras con Open Food Facts bajo consentimiento.
* Cuerpo: peso, tendencia, cinta métrica, detector de falsa subida, metas intermedias y bloques de actividad.
* Cuidado: tratamientos y tomas sin recompensas, hábitos, pausas activas y descanso visual 20-20-20 con avisos de
  escritorio, respiración y suelo pélvico guiados, signos vitales, síntomas, chequeos, alcohol y tabaco, y contadores
  para dejar hábitos.
* Ayuno con protocolos, comprobaciones de seguridad, cronómetro e historial.
* Nibi, un compañero con monedas locales, escudos de racha, tienda y armario sin pagos.
* Perfil: tu semana frente a la anterior, tendencias de hasta 180 días, patrones descriptivos entre hábitos, historial
  corregible de cualquier origen, objetivos adaptativos, plan semanal, ciclo opcional y resumen imprimible.
* Sincronización en vivo con NB SelfCare Mobile por la red local: vinculación con código QR o en texto, TLS con
  certificado fijado, credenciales revocables en el llavero, identidades estables, cambios capturados en la misma
  transacción y revisión en el escritorio de los cambios simultáneos. El esquema de datos es el mismo en los dos
  equipos.
* Interfaz en español, inglés y portugués, con cambio al momento; 24 temas con Modo Papel, cinco tipografías y escala
  de texto.
* Integración con NB Launcher: distribución firmada e instalación administrada, sin acceso a registros de salud.
* Una sola puerta de red con interruptor general y consentimiento por servicio; sin telemetría.
* Copias de seguridad en un archivo SQLite compatible con el teléfono, con copias automáticas y una copia previa a
  cada restauración y migración.
* Paquetes para Linux (`.deb`, `.rpm`, `.tar.gz` y AppImage), verificados en un equipo real. Paquetes **sin probar en
  un equipo real** para Windows (`-setup.exe` y `.msi`) y macOS en Apple Silicon (`.dmg`); no llevan firma de
  Microsoft ni de Apple, así que Windows muestra el aviso de SmartScreen y macOS pide autorizar la aplicación al
  abrirla. Todos firmados para el actualizador y en una lista de huellas `SHA256SUMS` firmada con Ed25519.
