/** Comprueba el despacho real al servidor local de notificaciones, con contenido sintético discreto. */
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {basename} from 'node:path';
import {readlinkSync} from 'node:fs';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector,esperar} from './inspector.mjs';
verificarAislamiento();
const pid=execFileSync('ss',['-ltnp','( sport = :9333 )'],{encoding:'utf8'}).match(/pid=(\d+)/)?.[1];assert.ok(pid);
const nombre=basename(readlinkSync(`/proc/${pid}/exe`));assert.match(nombre,/^nb-selfcare-qa-/);
execFileSync('gdbus',['call','--session','--dest','org.freedesktop.Notifications','--object-path','/org/freedesktop/Notifications','--method','org.freedesktop.Notifications.GetServerInformation'],{stdio:'pipe'});
const monitor=spawn('dbus-monitor',['--session',`type='method_call',interface='org.freedesktop.Notifications',member='Notify',arg0='${nombre}'`],{stdio:['ignore','pipe','pipe']});
let recibido='';monitor.stdout.on('data',d=>recibido+=d);monitor.stderr.resume();
const i=await Inspector.conectar();let id;
try{
 await esperar(300);
 id=await i.evaluar(`(()=>{const ahora=new Date();return window.__TAURI_INTERNALS__.invoke('tratamiento_guardar',{id:null,tratamiento:{nombre:'Contenido privado sintético QA',dosis:'Demostración',tipo:'medication',horas:[ahora.getHours()*60+ahora.getMinutes()],dias:127,reserva:null,unidadesPorToma:1,avisos:true,bajaGlucosa:false,retieneAgua:false,notas:null}});})()`);
 console.log('Esperando el ciclo real del planificador de avisos.');
 for(let n=0;n<90&&!recibido.includes('Es la hora de tu toma');n++)await esperar(500);
 assert.ok(recibido.includes('Es la hora de tu toma'),'No se observó el despacho al servidor de notificaciones');
 assert.ok(!recibido.includes('Contenido privado sintético QA'),'El aviso discreto no debe contener el nombre del tratamiento');
 console.log('Correcto: aviso real enviado al servidor local sin revelar el nombre del tratamiento');
}finally{
 if(id!==undefined)await i.evaluar(`window.__TAURI_INTERNALS__.invoke('tratamiento_estado',{id:${id},activo:null})`);
 i.cerrar();monitor.kill();
}
