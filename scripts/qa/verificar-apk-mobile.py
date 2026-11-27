#!/usr/bin/env python3
"""Verifica el APK construido; no instala, descubre ni conecta dispositivos."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

raiz = Path(__file__).resolve().parents[2]
movil = Path(os.environ['NB_SELFCARE_MOBILE_DIR'])
assert movil.is_absolute() and (movil / 'pubspec.yaml').is_file()
propiedades = dict(linea.split('=', 1) for linea in (movil / 'android/local.properties').read_text().splitlines() if '=' in linea)
versiones = sorted((Path(propiedades['sdk.dir']) / 'build-tools').iterdir(), key=lambda p: tuple(int(x) for x in re.findall(r'\d+', p.name)))
herramientas = next(p for p in reversed(versiones) if (p / 'apksigner').exists() and (p / 'aapt2').exists())
apk = movil / 'build/app/outputs/flutter-apk/app-release.apk'

def ejecutar(argumentos):
    return subprocess.run(argumentos, capture_output=True, text=True, check=True).stdout

verificacion = ejecutar([str(herramientas / 'apksigner'), 'verify', '--verbose', '--print-certs', str(apk)])
certificado = re.search(r'Signer #1 certificate SHA-256 digest: ([a-fA-F0-9]+)', verificacion).group(1).lower()
politica = (movil / 'docs/audit/supply-chain.md').read_text()
referencia = re.search(r'`([0-9A-F]{2}(?::[0-9A-F]{2}){31})`', politica).group(1).replace(':', '').lower()
assert certificado == referencia, 'El certificado no coincide con la política del móvil'
datos = ejecutar([str(herramientas / 'aapt2'), 'dump', 'badging', str(apk)])
paquete = re.search(r"package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'", datos)
assert paquete.group(1) == 'com.nb.selfcare'
version_esperada = re.search(r'^version:\s*(\S+)', (movil / 'pubspec.yaml').read_text(), re.MULTILINE).group(1)
assert f'{paquete.group(3)}+{paquete.group(2)}' == version_esperada, 'El APK no corresponde a la versión fuente'
permisos = ejecutar([str(herramientas / 'aapt2'), 'dump', 'permissions', str(apk)])
assert 'android.permission.CHANGE_WIFI_MULTICAST_STATE' in permisos
mapeo = (movil / 'build/app/outputs/mapping/release/mapping.txt').read_text()
assert 'com.haberey.flutter.nsd_android.NsdAndroidPlugin ->' in mapeo
with zipfile.ZipFile(apk) as contenido:
    abis = sorted({n.split('/')[1] for n in contenido.namelist() if n.startswith('lib/') and len(n.split('/')) > 2})
    assert abis == ['arm64-v8a', 'armeabi-v7a', 'x86_64']
    assert not any(n.endswith(('.jks', '.keystore', 'key.properties')) for n in contenido.namelist())
    assert contenido.read('assets/flutter_assets/assets/fonts/Manrope.ttf') == (movil / 'assets/fonts/Manrope.ttf').read_bytes(), 'Falta la fuente empaquetada del informe'
archivos = [
    'lib/core/services/backup_service.dart',
    'lib/core/database/respaldo_de_migracion.dart',
    'lib/core/network/sincronizacion/reconexion.dart',
    'lib/core/network/sincronizacion/descubrimiento.dart',
    'lib/core/network/sincronizacion/almacen_del_sistema.dart',
    'lib/core/services/clinical_report_pdf_service.dart',
    'lib/core/sync/contrato.dart',
    'lib/core/sync/huella.dart',
]
fuentes = {p: hashlib.sha256((movil / p).read_bytes()).hexdigest() for p in archivos}
with apk.open('rb') as archivo:
    resumen = hashlib.sha256()
    for bloque in iter(lambda: archivo.read(1024 * 1024), b''):
        resumen.update(bloque)
    huella = resumen.hexdigest()
resultado = {
    'paquete': paquete.group(1), 'version': f'{paquete.group(3)}+{paquete.group(2)}',
    'bytes': apk.stat().st_size, 'sha256': huella, 'certificadoSha256': certificado,
    'abis': abis, 'firmaVerificada': True, 'instaladoEnTelefono': False,
    'incluyeNsd': True, 'permisoMulticastVerificado': True,
    'fuentePdfEmpaquetadaVerificada': True,
    'huellasDeFuentes': fuentes,
    'limites': 'No acredita almacén seguro ni mDNS en teléfono. Sin aplicación de registros sanitarios ni flujo de conexión en la interfaz.',
}
salida = raiz / 'test-results/mobile-apk/resultado.json'
salida.parent.mkdir(parents=True, exist_ok=True)
salida.write_text(json.dumps(resultado, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({k: resultado[k] for k in ['paquete', 'version', 'bytes', 'sha256', 'firmaVerificada', 'instaladoEnTelefono']}, ensure_ascii=False))
