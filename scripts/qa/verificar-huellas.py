#!/usr/bin/env python3
"""Comprueba vectores sintéticos con una implementación independiente; no pertenece al runtime."""
import hashlib
import json
import math
import os
from pathlib import Path
import struct


def codificar(valor):
    if valor is None:
        return b'\x00'
    if valor is False:
        return b'\x01'
    if valor is True:
        return b'\x02'
    if isinstance(valor, (int, float)):
        if not math.isfinite(valor) or abs(valor) > 9007199254740991:
            raise ValueError('Número fuera del contrato')
        return b'\x03' + struct.pack('>d', 0.0 if valor == 0 else float(valor))
    if isinstance(valor, str):
        texto = valor.encode('utf-8', errors='strict')
        return b'\x04' + struct.pack('>I', len(texto)) + texto
    if isinstance(valor, list):
        return b'\x05' + struct.pack('>I', len(valor)) + b''.join(map(codificar, valor))
    if isinstance(valor, dict) and all(isinstance(clave, str) for clave in valor):
        claves = sorted(valor, key=lambda clave: clave.encode('utf-8', errors='strict'))
        return b'\x06' + struct.pack('>I', len(valor)) + b''.join(
            codificar(clave) + codificar(valor[clave]) for clave in claves
        )
    raise ValueError('Tipo fuera del contrato')


def main():
    raiz = Path(__file__).resolve().parents[2]
    movil = Path(os.environ['NB_SELFCARE_MOBILE_DIR'])
    if not movil.is_absolute():
        raise ValueError('La carpeta móvil debe ser absoluta')
    compartidos = ['lote-v1.json', 'invitacion-v1.json', 'huellas-v1.json', 'recepcion-v1.sql']
    for nombre in compartidos:
        escritorio = raiz / 'src-tauri/tests/fixtures/sincronizacion' / nombre
        contraparte = movil / 'test/fixtures/sincronizacion' / nombre
        if escritorio.read_bytes() != contraparte.read_bytes():
            raise ValueError('Los propietarios conservan fixtures diferentes')
    contenido = (raiz / 'src-tauri/tests/fixtures/sincronizacion/huellas-v1.json').read_bytes()
    casos = json.loads(contenido)
    for caso in casos:
        representacion = codificar({'nodo': caso['nodo'], 'cambio': caso['cambio']})
        huella = hashlib.sha256(b'NB-SelfCare/operacion/1\x00' + representacion).hexdigest()
        if huella != caso['sha256']:
            raise ValueError('Un vector no coincide con la codificación independiente')
    print(f'Correcto: {len(casos)} vectores independientes y {len(compartidos)} fixtures idénticos en ambos propietarios')


if __name__ == '__main__':
    main()
