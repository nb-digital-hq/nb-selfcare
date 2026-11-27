#!/usr/bin/env python3
"""Limpieza local en Linux de artefactos ignorados conocidos de los tres proyectos."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess


def fuentes(repo):
    salida = subprocess.run(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'],
        cwd=repo, check=True, capture_output=True,
    ).stdout
    resultado = {}
    for nombre in set(salida.decode().split('\0')) - {''}:
        archivo = repo / nombre
        if archivo.is_symlink():
            resultado[nombre] = ('enlace', str(archivo.readlink()))
        elif archivo.is_file():
            resultado[nombre] = hashlib.sha256(archivo.read_bytes()).hexdigest()
    return resultado


def variantes_superadas(carpeta):
    grupos = {}
    if not carpeta.is_dir():
        return []
    for paquete in carpeta.iterdir():
        if not paquete.is_dir() or paquete.is_symlink():
            continue
        for variante in paquete.iterdir():
            if not variante.is_dir() or variante.is_symlink() or not re.fullmatch(r'[0-9a-f]{16}', variante.name):
                continue
            archivos = list((variante / 'fingerprint').glob('*.json'))
            if len(archivos) != 1:
                continue
            try:
                dato = json.loads(archivos[0].read_text())
                clave = (paquete.name, archivos[0].name, dato['target'], dato['profile'],
                         dato['features'], dato['compile_kind'], json.dumps(dato['rustflags']))
                grupos.setdefault(clave, []).append((archivos[0].stat().st_mtime_ns, variante))
            except (ValueError, TypeError, KeyError, OSError):
                # Un formato desconocido se conserva; no se infiere que sea descartable.
                continue
    return [p for valores in grupos.values() for _, p in sorted(valores, reverse=True)[1:]]


def comprobar_inactividad(repos):
    if not Path('/proc').is_dir():
        raise RuntimeError('Esta herramienta de limpieza necesita el inventario de procesos de Linux')
    for proceso in Path('/proc').iterdir():
        if not proceso.name.isdigit():
            continue
        try:
            nombre = (proceso / 'comm').read_text().strip()
            if nombre not in {'cargo', 'rustc', 'flutter_tester', 'nb-selfcare', 'nb-launcher'}:
                continue
            cwd = (proceso / 'cwd').resolve()
            ejecutable = (proceso / 'exe').resolve()
            if any(cwd.is_relative_to(repo) or ejecutable.is_relative_to(repo) for repo in repos):
                raise RuntimeError('Hay un proceso del proyecto activo; termina su operación antes de limpiar')
        except (FileNotFoundError, PermissionError):
            continue


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--aplicar', action='store_true', help='Sin este argumento solo se prepara el inventario')
    argumentos = parser.parse_args()
    raiz = Path(__file__).resolve().parents[2]
    repos = {'NB_SelfCare': raiz, 'NB_SelfCare_Mobile': raiz.parent / 'NB_SelfCare_Mobile',
             'NB_Launcher': raiz.parent / 'NB_Launcher'}
    for repo in repos.values():
        if repo.is_symlink() or not (repo / '.git').exists():
            raise RuntimeError('No se encontró uno de los repositorios esperados')
    comprobar_inactividad(repos.values())
    antes = {nombre: fuentes(repo) for nombre, repo in repos.items()}
    candidatos = []
    for nombre, repo in repos.items():
        target = repo / ('src-tauri/target' if nombre == 'NB_SelfCare' else 'target')
        candidatos.extend((nombre, p) for p in variantes_superadas(target / 'debug/build'))
        candidatos.append((nombre, target / 'debug/incremental'))
    candidatos.extend(('NB_SelfCare', raiz / p) for p in [
        'src-tauri/target/entrega-local-linux', 'src-tauri/target/entrega-launcher-local',
        'src-tauri/target/flycheck0', 'scripts/qa/__pycache__',
    ])
    candidatos.extend(('NB_SelfCare_Mobile', repos['NB_SelfCare_Mobile'] / p) for p in [
        'build/test_cache', 'build/unit_test_assets',
    ])
    verificacion = raiz / 'test-results/verificacion'
    conservar = {
        'actionlint-final.log', 'apk-final.log', 'apk-verificacion.log', 'clippy-final.log',
        'deny-final.log', 'docs-final.log', 'escritorio-final.log', 'firmas-linux-final.log',
        'huellas-independientes.log', 'huellas-rust.log', 'huellas-dart.log',
        'interfaz-build-final.log', 'linux-empaquetado.log', 'linux-tar-final.log',
        'mobile-analyze.log', 'mobile-docs-final.log', 'mobile-final.log', 'mobile-formato.log',
        'mobile-guardianes-final.log', 'mobile-version-final.log', 'notices-final.log',
        'pdf-despues.log', 'pdf-renderizado.log', 'rust-final.log', 'tls-reconexion-dart-rust.log',
        'recepcion-rust.log', 'recepcion-dart.log', 'flutter-toolchain.log', 'limpieza-contenedores.log',
    }
    candidatos.extend(('NB_SelfCare', p) for p in verificacion.glob('*.log') if p.name not in conservar)
    candidatos.append(('NB_SelfCare', raiz / 'test-results/paquete-linux/aviso-observado.png'))
    informe = verificacion / ('limpieza-local.json' if argumentos.aplicar else 'limpieza-local-previa.json')
    anterior = json.loads(informe.read_text()) if argumentos.aplicar and informe.exists() else {}
    acciones = []
    espacio_antes = shutil.disk_usage(raiz).free
    for nombre, candidato in candidatos:
        repo = repos[nombre]
        if not candidato.exists():
            continue
        relativo = candidato.relative_to(repo)
        if any(p.is_symlink() for p in [candidato, *candidato.parents] if p != repo and p.is_relative_to(repo)):
            raise RuntimeError('Un artefacto candidato atraviesa un enlace simbólico')
        if any(p == str(relativo) or p.startswith(str(relativo) + '/') for p in antes[nombre]):
            raise RuntimeError('Un candidato contiene código fuente o archivos versionados')
        ignorado = subprocess.run(['git', 'check-ignore', '--no-index', '-q', str(relativo)], cwd=repo)
        if ignorado.returncode != 0:
            raise RuntimeError('Un candidato no está ignorado por Git')
        acciones.append({'repo': nombre, 'artefacto': str(relativo)})
        if argumentos.aplicar:
            if candidato.is_dir():
                shutil.rmtree(candidato)
            else:
                candidato.unlink()
    if any(fuentes(repo) != antes[nombre] for nombre, repo in repos.items()):
        raise RuntimeError('Cambió un archivo fuente durante la limpieza; revisar antes de continuar')
    resultado = {'aplicada': argumentos.aplicar, 'acciones': acciones,
                 'fuentesConservadas': True,
                 'diferenciaEspacioLibreBytes': shutil.disk_usage(raiz).free - espacio_antes}
    verificacion.mkdir(parents=True, exist_ok=True)
    if argumentos.aplicar:
        acumulado = anterior.get('totalEspacioRecuperadoBytes', max(0, anterior.get('diferenciaEspacioLibreBytes', 0))) if anterior.get('aplicada') else 0
        artefactos = anterior.get('totalArtefactosRetirados', len(anterior.get('acciones', []))) if anterior.get('aplicada') else 0
        resultado['totalEspacioRecuperadoBytes'] = acumulado + max(0, resultado['diferenciaEspacioLibreBytes'])
        resultado['totalArtefactosRetirados'] = artefactos + len(acciones)
    informe.write_text(json.dumps(resultado, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'aplicada': argumentos.aplicar, 'artefactos': len(acciones),
                      'fuentesConservadas': True,
                      'diferenciaEspacioLibreBytes': resultado['diferenciaEspacioLibreBytes']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
