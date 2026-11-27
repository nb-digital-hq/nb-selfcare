/** Exportación y restauración mediante los diálogos GTK reales, solo con datos sintéticos. */
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {existsSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector,esperar} from './inspector.mjs';
const base=verificarAislamiento();
const pid=execFileSync('ss',['-ltnp','( sport = :9333 )'],{encoding:'utf8'}).match(/pid=(\d+)/)?.[1];assert.ok(pid);
const copia=join(base,`copia-dialogo-${Date.now()}.sqlite`);
const i=await Inspector.conectar();
const hasta=async expresion=>{for(let n=0;n<120;n++){if(await i.evaluar(expresion))return;await esperar(100);}throw Error('No se confirmó el flujo de copias');};
const navegar=async ruta=>{await i.evaluar(`document.querySelector('nav a[href="${ruta}"]').click()`);await hasta(`(location.pathname||'/')===${JSON.stringify(ruta)}&&!document.querySelector('main .carga')`);await esperar(150);};
const boton=async texto=>{await hasta(`[...document.querySelectorAll('main button')].some(e=>e.textContent.trim()===${JSON.stringify(texto)}&&!e.disabled)`);await i.evaluar(`[...document.querySelectorAll('main button')].find(e=>e.textContent.trim()===${JSON.stringify(texto)}).click()`);};
const dialogo=(accion)=>execFileSync('python',[fileURLToPath(new URL('./dialogo-nativo.py',import.meta.url)),pid,accion,copia],{stdio:'pipe',timeout:20000});
try{
 await i.instalarCapturaDeErrores();
 await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 await navegar('/ajustes');await i.evaluar(`document.getElementById('ajuste-copias').click()`);await boton('Exportar copia');dialogo('guardar');
 for(let n=0;n<100&&!existsSync(copia);n++)await esperar(100);assert.ok(existsSync(copia));
 console.log('Correcto: exportación por selector nativo');
 await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 await i.evaluar(`(()=>{const e=document.querySelector('main input');e.value='Cambio temporal';e.dispatchEvent(new Event('input',{bubbles:true}));e.form.requestSubmit();})()`);
 await hasta(`document.querySelector('header')?.textContent!==null`);await esperar(400);
 await navegar('/');await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Cambio temporal'`);
 await navegar('/ajustes');await i.evaluar(`document.getElementById('ajuste-copias').click()`);await boton('Elegir copia de escritorio o móvil');dialogo('abrir');
 await hasta(`document.querySelector('.resultado strong')?.textContent==='Demostración'`);
 await boton('Restaurar copia');await hasta(`!!document.querySelector('dialog[open] .confirmar')`);
 await i.evaluar(`document.querySelector('dialog[open] .confirmar').click()`);
 await hasta(`!document.querySelector('dialog[open]')`);
 await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 console.log('Correcto: selección, inspección, confirmación y restauración; formulario con datos recuperados');
 await navegar('/ajustes');await i.evaluar(`document.getElementById('ajuste-copias').click()`);await boton('Elegir copia de escritorio o móvil');dialogo('cancelar');
 await esperar(300);await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 console.log('Correcto: cancelar el selector conserva el perfil');
 assert.deepEqual(await i.errores(),[]);
}finally{i.cerrar();}
