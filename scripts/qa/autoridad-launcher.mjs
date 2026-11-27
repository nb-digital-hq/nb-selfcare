/** Reinicia únicamente la aplicación aislada de QA para comprobar ambas autoridades. */
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {readFileSync,readlinkSync,mkdtempSync,mkdirSync,copyFileSync,writeFileSync,openSync,closeSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector,esperar} from './inspector.mjs';
verificarAislamiento();
const propietario=()=>execFileSync('ss',['-ltnp','( sport = :9333 )'],{encoding:'utf8'}).match(/pid=(\d+)/)?.[1];
const pid=Number(propietario());assert.ok(pid);
const ejecutable=readlinkSync(`/proc/${pid}/exe`);
assert.ok(ejecutable.startsWith('/tmp/nb-selfcare-qa-'));
const env=Object.fromEntries(readFileSync(`/proc/${pid}/environ`,'utf8').split('\0').filter(e=>e.includes('=')).map(e=>{const n=e.indexOf('=');return [e.slice(0,n),e.slice(n+1)];}));
const carpeta=mkdtempSync(join(tmpdir(),'nb-selfcare-qa-managed-'));
mkdirSync(join(carpeta,'bin'));
const administrado=join(carpeta,'bin','nb-selfcare');copyFileSync(ejecutable,administrado);
writeFileSync(join(carpeta,'bin','.nb-launcher-managed'),'nb-launcher-managed-v1\n');
async function parar(id){process.kill(Number(id),'SIGTERM');for(let n=0;n<50&&propietario()===String(id);n++)await esperar(100);assert.notEqual(propietario(),String(id));}
function iniciar(ruta){const log=openSync(join(carpeta,'app.log'),'a');const p=spawn(ruta,[],{env,stdio:['ignore',log,log],detached:true});closeSync(log);p.unref();return p;}
async function preparada(inspector){for(let n=0;n<100;n++){try{if(await inspector.evaluar(`typeof window.__TAURI_INTERNALS__?.invoke==='function'&&!!document.querySelector('nav')`))return;}catch(error){if(!String(error).includes('Missing injected script'))throw error;}await esperar(100);}throw Error('La aplicación QA no terminó de iniciar');}
await parar(pid);
const p=iniciar(administrado);let i;
try{
 i=await Inspector.conectar();
 await preparada(i);
 assert.equal(await i.evaluar(`window.__TAURI_INTERNALS__.invoke('salud').then(s=>s.gestionadaPorLauncher)`),true);
 await i.evaluar(`document.querySelector('nav a[href="/ajustes"]').click()`);
 for(let n=0;n<100;n++){if(await i.evaluar(`!!document.getElementById('ajuste-actualizaciones')`))break;await esperar(100);}
 await i.evaluar(`document.getElementById('ajuste-actualizaciones').click()`);
 let visible=false;
 for(let n=0;n<100;n++){visible=await i.evaluar(`document.querySelector('main')?.textContent.includes('NB Launcher administra las actualizaciones de esta copia.')`);if(visible)break;await esperar(100);}
 assert.ok(visible);
 assert.equal(await i.evaluar(`[...document.querySelectorAll('main button')].some(e=>e.textContent.trim()==='Buscar actualización')`),false);
 for(const comando of ['actualizacion_buscar','actualizacion_instalar']){
  assert.equal(await i.evaluar(`window.__TAURI_INTERNALS__.invoke('${comando}').then(()=>false,e=>e==='COMPONENT_UNAVAILABLE'||e?.code==='COMPONENT_UNAVAILABLE')`),true);
 }
 console.log('Correcto: la copia administrada informa autoridad, muestra Launcher y rechaza ambos actualizadores antes de salir a la red');
}finally{
 i?.cerrar();await parar(p.pid);
 const vuelta=iniciar(ejecutable);writeFileSync('/tmp/nb-selfcare-qa-launcher.pid',String(vuelta.pid));
}
i=await Inspector.conectar();try{await preparada(i);assert.equal(await i.evaluar(`window.__TAURI_INTERNALS__.invoke('salud').then(s=>s.gestionadaPorLauncher)`),false);console.log('Correcto: reinicio independiente sin marca conserva su propio actualizador');}finally{i.cerrar();}
