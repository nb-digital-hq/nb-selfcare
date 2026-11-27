#!/usr/bin/env python3
"""Accesibilidad de diálogos GTK del proceso QA; no forma parte del runtime."""
import os
import sys
import tempfile
import time
from pathlib import Path

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

pid, accion = int(sys.argv[1]), sys.argv[2]
entorno = dict(e.split(b"=", 1) for e in Path(f"/proc/{pid}/environ").read_bytes().split(b"\0") if b"=" in e)
base = Path(os.fsdecode(entorno[b"NB_SELFCARE_TEST_DIR"])).resolve()
assert base.parent == Path(tempfile.gettempdir()).resolve() and base.name.startswith("nb-selfcare-qa-")
ruta = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
if ruta:
    assert ruta.is_relative_to(base), "El archivo debe pertenecer a los datos sintéticos aislados"

portal_permitido = (Path('/run/.containerenv').exists() and os.geteuid() == 0
                    and Path(f'/proc/{pid}/exe').resolve() == Path('/opt/nb-selfcare-qa/usr/bin/nb-selfcare'))

def es_proceso_permitido(app):
    proceso = app.get_process_id()
    if proceso == pid:
        return True
    # Ubuntu puede alojar el selector en su portal GTK. Solo se permite en el
    # contenedor privado y con el ejecutable conocido, nunca en la sesión anfitriona.
    return portal_permitido and Path(f'/proc/{proceso}/exe').resolve() == Path('/usr/libexec/xdg-desktop-portal-gtk')

def descendientes(elemento):
    yield elemento
    if elemento.get_role_name() in {"table", "list", "list box", "tree table", "tree"}:
        return
    for n in range(min(elemento.get_child_count(), 80)):
        yield from descendientes(elemento.get_child_at_index(n))

def visible(elemento):
    return elemento.get_state_set().contains(Atspi.StateType.SHOWING)

def dialogo():
    escritorio = Atspi.get_desktop(0)
    for n in range(escritorio.get_child_count()):
        app = escritorio.get_child_at_index(n)
        if es_proceso_permitido(app):
            for j in range(app.get_child_count() - 1, -1, -1):
                ventana = app.get_child_at_index(j)
                if (ventana.get_role_name() == "file chooser" and visible(ventana)
                    and ventana.get_name() == ("Guardar una copia de seguridad" if accion == "guardar" else "Restaurar una copia de seguridad")):
                    return ventana
    return None

limite = time.monotonic() + 12
while not (ventana := dialogo()):
    if time.monotonic() > limite:
        raise RuntimeError("No apareció el diálogo del proceso QA")
    time.sleep(.1)

if accion == "abrir":
    widget = next(e for e in descendientes(ventana) if e.get_role_name() == "file chooser" and e.get_action_iface())
    assert widget.get_action_iface().do_action(0)
    time.sleep(.2)
if accion in {"guardar", "abrir"}:
    entradas = [e for e in descendientes(ventana) if visible(e) and e.get_editable_text_iface()]
    assert len(entradas) == 1
    assert entradas[0].get_editable_text_iface().set_text_contents(str(ruta))
    if accion == "abrir":
        time.sleep(.3)
        abrir = next(e for e in descendientes(ventana) if e.get_role() == Atspi.Role.PUSH_BUTTON and e.get_name() == "Open" and visible(e))
        assert Atspi.Action.do_action(abrir, 0)
        time.sleep(.3)
        Atspi.Action.do_action(entradas[0], 0)
        print("Diálogo nativo QA: ruta sintética activada")
        sys.exit(0)
    etiqueta = "Save"
elif accion == "cancelar":
    etiqueta = "Cancel"
else:
    raise RuntimeError("Acción no admitida")
boton = next(e for e in descendientes(ventana) if e.get_role() == Atspi.Role.PUSH_BUTTON and e.get_name() == etiqueta and visible(e))
assert boton.get_action_iface().do_action(0)
print("Diálogo nativo QA: acción enviada al proceso aislado")
