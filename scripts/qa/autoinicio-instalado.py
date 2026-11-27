#!/usr/bin/env python3
"""Autoinicio y copias nativas exclusivamente dentro de un contenedor desechable."""
import os
import subprocess
import tempfile
import sqlite3
import json
import sys
import shutil
import time
from pathlib import Path
import gi

gi.require_version('Atspi', '2.0')
from gi.repository import Atspi, Gio, GLib

assert Path('/run/.containerenv').exists() and os.geteuid() == 0
binario = os.environ.get('NB_QA_BINARIO')
assert binario == '/opt/nb-selfcare-qa/usr/bin/nb-selfcare'
marca = Path.home() / '.config/autostart/NB SelfCare.desktop'
assert not marca.exists()
base = Path(tempfile.mkdtemp(prefix='nb-selfcare-qa-autoinicio-'))
Path('/qa-salida/aviso-geometria.json').unlink(missing_ok=True)
# La bienvenida ocupa todos los destinos hasta crear el perfil. Este fixture solo aísla la prueba de autoinicio.
(base / 'datos').mkdir()
with sqlite3.connect('/tmp/nb-selfcare-qa-instalada/datos/nb-selfcare.sqlite') as origen:
    with sqlite3.connect(base / 'datos/nb-selfcare.sqlite') as destino:
        origen.backup(destino)
        destino.execute('BEGIN IMMEDIATE')
        assert destino.execute('SELECT count(*) FROM user_profiles').fetchone()[0] == 0
        destino.execute("INSERT INTO user_profiles(name,birth_date,biological_sex,height_cm,target_weight_kg) VALUES(?,?,?,?,?)", ('Demostración', '1990-01-01T00:00:00.000 +00:00', 'male', 170, 70))
        destino.execute('INSERT OR REPLACE INTO preferencias(clave,valor) VALUES(?,?)', ('idioma', '"es"'))
env = os.environ.copy()
env.update(NB_SELFCARE_TEST_DIR=str(base), NO_AT_BRIDGE='0', GTK_MODULES='atk-bridge', WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS='1')

def recorrer(e, profundidad=0):
    if e is None:
        return
    try:
        e.clear_cache()
        cantidad = e.get_child_count()
    except GLib.Error:
        return
    yield e
    if profundidad < 24:
        for n in range(min(cantidad, 120)):
            try:
                hijo = e.get_child_at_index(n)
            except GLib.Error:
                continue
            yield from recorrer(hijo, profundidad + 1)

def buscar(nombre, rol):
    escritorio = Atspi.get_desktop(0)
    escritorio.clear_cache()
    for n in range(escritorio.get_child_count()):
        app = escritorio.get_child_at_index(n)
        if app.get_process_id() == proceso.pid:
            for e in recorrer(app):
                coincide = e.get_role_name() == rol or (rol == 'button' and e.get_role() == Atspi.Role.PUSH_BUTTON)
                if e.get_name().strip() in nombre and (coincide or (rol in ("link", "page tab") and e.get_action_iface() and e.get_action_iface().get_n_actions() > 0)):
                    return e
    return None

def hasta(funcion):
    limite = time.monotonic() + 30
    while time.monotonic() < limite:
        if proceso.poll() is not None:
            raise RuntimeError('La aplicación instalada terminó durante el recorrido')
        contexto = GLib.MainContext.default()
        while contexto.pending():
            contexto.iteration(False)
        try:
            valor = funcion()
        except GLib.Error:
            # Los nodos de accesibilidad se sustituyen al cambiar una vista.
            valor = None
        if valor:
            return valor
        time.sleep(.15)
    raise RuntimeError('No se confirmó el recorrido del tar instalado')

def activar(nombres, rol='button'):
    elemento = hasta(lambda: buscar(nombres, rol))
    # La acción accesible no simula el foco que precede a una activación por teclado.
    assert Atspi.Component.grab_focus(elemento)
    assert Atspi.Action.do_action(elemento, 0)

def confirmar_restauracion():
    modal = hasta(lambda: buscar(('Restaurar copia',), 'dialog'))
    boton = next(e for e in recorrer(modal) if e.get_role() == Atspi.Role.PUSH_BUTTON and e.get_name() == 'Restaurar copia')
    assert Atspi.Component.grab_focus(boton)
    assert Atspi.Action.do_action(boton, 0)

def elegir_archivo(accion, ruta):
    assert ruta.is_relative_to(base)
    subprocess.run([sys.executable, '/qa-dialogo.py', str(proceso.pid), accion, str(ruta)], check=True, timeout=25)

def nombre_guardado(archivo):
    with sqlite3.connect(archivo) as db:
        return db.execute('SELECT name FROM user_profiles ORDER BY id LIMIT 1').fetchone()[0]

def contiene_texto_visible(texto):
    escritorio = Atspi.get_desktop(0)
    for n in range(escritorio.get_child_count()):
        app = escritorio.get_child_at_index(n)
        if app.get_process_id() == proceso.pid:
            marco = Atspi.Component.get_extents(app.get_child_at_index(0), Atspi.CoordType.SCREEN)
            mensaje_encontrado = False
            region = None
            for e in recorrer(app):
                if e.get_name() == 'Problemas' and e.get_role_name() == 'landmark':
                    region = e
                interfaz = e.get_text_iface()
                contenido = interfaz.get_text(0, -1) if interfaz else e.get_name()
                if texto in contenido:
                    mensaje_encontrado = True
            # WebKit agrupa el texto en la página y devuelve rangos de tamaño cero.
            # La región viva tiene caja propia; la captura comprueba además su texto dibujado.
            if mensaje_encontrado and region is not None:
                rect = Atspi.Component.get_extents(region, Atspi.CoordType.SCREEN)
                ventana = Gdk.get_default_root_window()
                dentro = rect.width > 0 and rect.height > 0 and rect.x >= max(0, marco.x) and rect.y >= max(0, marco.y) and rect.x + rect.width <= min(ventana.get_width(), marco.x + marco.width) and rect.y + rect.height <= min(ventana.get_height(), marco.y + marco.height)
                Path('/qa-salida/aviso-geometria.json').write_text(json.dumps({
                    'regionDelAviso': {'x': rect.x, 'y': rect.y, 'ancho': rect.width, 'alto': rect.height},
                    'ventana': {'x': marco.x, 'y': marco.y, 'ancho': marco.width, 'alto': marco.height},
                    'mensajeEnArbolAccesible': True,
                    'dentroDeVentana': dentro,
                }, indent=2) + '\n')
                if dentro:
                    return True
    return False

def probar_copias():
    activar(('Copias de seguridad',), 'page tab')
    copia = base / 'copia-qa.sqlite'
    activar(('Exportar copia',))
    elegir_archivo('guardar', copia)
    hasta(copia.exists)
    assert nombre_guardado(copia) == 'Demostración'
    viva = base / 'datos/nb-selfcare.sqlite'
    with sqlite3.connect(viva) as db:
        db.execute('BEGIN IMMEDIATE')
        db.execute("UPDATE user_profiles SET name = 'Estado previo'")
    activar(('Elegir copia de escritorio o móvil',))
    elegir_archivo('abrir', copia)
    activar(('Restaurar copia',))
    confirmar_restauracion()
    hasta(lambda: nombre_guardado(viva) == 'Demostración')
    hasta(lambda: buscar(('Restaurar copia',), 'dialog') is None)
    previas = list((base / 'datos/copias').glob('antes-de-restaurar-*.sqlite'))
    assert len(previas) == 1 and nombre_guardado(previas[0]) == 'Estado previo'
    print('Correcto: tar instalado exporta, inspecciona y restaura por diálogos nativos, conservando el estado anterior', flush=True)
    incompleta = base / 'incompleta.sqlite'
    shutil.copyfile(copia, incompleta)
    with sqlite3.connect(incompleta) as db:
        db.execute('DROP TABLE hydration_logs')
    with sqlite3.connect(viva) as db:
        db.execute('BEGIN IMMEDIATE')
        db.execute("INSERT INTO hydration_logs(date_key,logged_at,volume_ml) VALUES('2026-11-25','2026-11-25T12:00:00',321)")
    activar(('Copias de seguridad',), 'page tab')
    activar(('Elegir copia de escritorio o móvil',))
    elegir_archivo('abrir', incompleta)
    activar(('Restaurar copia',))
    confirmar_restauracion()
    hasta(lambda: contiene_texto_visible('El archivo no es una copia compatible de SelfCare.'))
    hasta(lambda: buscar(('Restaurar copia',), 'dialog') is None)
    hasta(lambda: (boton := buscar(('Elegir copia de escritorio o móvil',), 'button')) and boton.get_state_set().contains(Atspi.StateType.FOCUSED))
    assert nombre_guardado(viva) == 'Demostración'
    with sqlite3.connect(viva) as db:
        assert db.execute('SELECT sum(volume_ml) FROM hydration_logs').fetchone()[0] == 321
    ventana = Gdk.get_default_root_window()
    pix = Gdk.pixbuf_get_from_window(ventana, 0, 0, ventana.get_width(), ventana.get_height())
    pix.savev('/qa-salida/restauracion-rechazada.png', 'png', [], [])
    print('Correcto: tar instalado rechaza una copia incompleta sin sustituir los datos locales', flush=True)

ajustes_accesibilidad = Gio.Settings.new('org.gnome.desktop.interface')
ajustes_accesibilidad.set_boolean('toolkit-accessibility', True)
Gio.Settings.sync()
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
estado = Gio.DBusProxy.new_sync(bus, Gio.DBusProxyFlags.NONE, None, 'org.a11y.Bus', '/org/a11y/bus', 'org.a11y.Status', None)
for propiedad in ['IsEnabled', 'ScreenReaderEnabled']:
    if propiedad in (estado.get_cached_property_names() or []):
        bus.call_sync('org.a11y.Bus', '/org/a11y/bus', 'org.freedesktop.DBus.Properties', 'Set', GLib.Variant('(ssv)', ('org.a11y.Status', propiedad, GLib.Variant('b', True))), None, Gio.DBusCallFlags.NONE, 5000, None)
with (base / 'app.log').open('w') as log:
    proceso = subprocess.Popen([binario], env=env, stdout=log, stderr=subprocess.STDOUT)
try:
    ajustes = hasta(lambda: buscar(('Ajustes', 'Settings'), 'link'))
    assert Atspi.Component.grab_focus(ajustes)
    assert Atspi.Action.do_action(ajustes, 0)
    avisos = hasta(lambda: buscar(('Avisos de escritorio', 'Desktop reminders'), 'page tab'))
    assert Atspi.Action.do_action(avisos, 0)
    casilla = hasta(lambda: buscar(('Abrir al iniciar sesión', 'Open at login'), 'check box'))
    assert not casilla.get_state_set().contains(Atspi.StateType.CHECKED)
    assert Atspi.Action.do_action(casilla, 0)
    hasta(marca.exists)
    contenido = marca.read_text()
    assert binario in contenido and '--en-segundo-plano' in contenido
    casilla = hasta(lambda: buscar(('Abrir al iniciar sesión', 'Open at login'), 'check box'))
    hasta(lambda: casilla.get_state_set().contains(Atspi.StateType.CHECKED))
    gi.require_version('Gdk', '3.0')
    from gi.repository import Gdk
    escritorio = Atspi.get_desktop(0)
    app = next(escritorio.get_child_at_index(n) for n in range(escritorio.get_child_count()) if escritorio.get_child_at_index(n).get_process_id() == proceso.pid)
    frame = app.get_child_at_index(0)
    rect = Atspi.Component.get_extents(frame, Atspi.CoordType.SCREEN)
    raiz = Gdk.get_default_root_window()
    x, y = max(0, rect.x), max(0, rect.y)
    ancho, alto = min(rect.width, raiz.get_width() - x), min(rect.height, raiz.get_height() - y)
    pix = Gdk.pixbuf_get_from_window(raiz, x, y, ancho, alto)
    pix.savev('/qa-salida/autoinicio-configurado.png', 'png', [], [])
    assert Atspi.Action.do_action(casilla, 0)
    hasta(lambda: not marca.exists())
    probar_copias()
    Path('/qa-salida/fallo-autoinicio.png').unlink(missing_ok=True)
    print('Correcto: autoinicio instalado creado y retirado desde la interfaz nativa aislada')
except Exception:
    print((base / 'app.log').read_text(), flush=True)
    escritorio = Atspi.get_desktop(0)
    print('Aplicaciones del contenedor:', [(escritorio.get_child_at_index(n).get_name(), escritorio.get_child_at_index(n).get_process_id()) for n in range(escritorio.get_child_count())], 'PID esperado', proceso.pid, flush=True)
    for n in range(escritorio.get_child_count()):
        app = escritorio.get_child_at_index(n)
        if app.get_process_id() == proceso.pid:
            print('Árbol QA:', [(e.get_role_name(), e.get_name(), e.get_child_count()) for j, e in enumerate(recorrer(app)) if j < 100], flush=True)
    gi.require_version('Gdk', '3.0')
    from gi.repository import Gdk
    ventana = Gdk.get_default_root_window()
    pix = Gdk.pixbuf_get_from_window(ventana, 0, 0, ventana.get_width(), ventana.get_height())
    pix.savev('/qa-salida/fallo-autoinicio.png', 'png', [], [])
    raise
finally:
    proceso.terminate()
    try:
        proceso.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proceso.kill()
        proceso.wait()
