#!/usr/bin/env python3
"""Comprueba y renderiza únicamente los informes sintéticos del recorrido de QA móvil."""
from pathlib import Path
import subprocess


def main():
    carpeta = Path(__file__).resolve().parents[2] / 'test-results/pdf'
    for idioma, textos in {
        'es': ['7,8 h/noche', '74,0 kg', '2.120 kcal'],
        'en': ['7.8 h/night', '74.0 kg', '2,120 kcal'],
        'pt': ['7,8 h/noite', '74,0 kg', '2.120 kcal'],
    }.items():
        archivo = carpeta / f'informe-{idioma}.pdf'
        informacion = subprocess.run(['pdfinfo', str(archivo)], capture_output=True, text=True, check=True).stdout
        if not any(linea.startswith('Pages:') and linea.split(':')[1].strip() == '1' for linea in informacion.splitlines()):
            raise ValueError('El caso sintético debe ocupar una página')
        if '(A4)' not in informacion:
            raise ValueError('El informe sintético debe conservar el formato A4')
        texto = subprocess.run(['pdftotext', '-layout', str(archivo), '-'], capture_output=True, text=True, check=True).stdout
        for esperado in [*textos, 'niñez', 'alimentação', 'SpO₂ ≥ 95 %', '≤ 120 mmHg', 'Energía → recuperación']:
            if esperado not in texto:
                raise ValueError(f'El informe sintético {idioma} perdió un glifo o su formato numérico')
        (carpeta / f'informe-{idioma}.txt').write_text(texto)
        subprocess.run(['pdftoppm', '-scale-to', '1400', '-singlefile', '-png', str(archivo), str(carpeta / f'informe-{idioma}')], check=True)
    print('Correcto: glifos, tres idiomas, formatos numéricos y tres páginas A4 renderizadas para revisión visual')


if __name__ == '__main__':
    main()
