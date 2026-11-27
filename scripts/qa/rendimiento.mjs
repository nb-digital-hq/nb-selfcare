/** Medición de navegación e IPC calientes sobre el perfil sintético del webview real. */
import assert from 'node:assert/strict';
import {mkdirSync, writeFileSync} from 'node:fs';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector} from './inspector.mjs';
verificarAislamiento();
const i=await Inspector.conectar();
try {
 const resultado=await i.evaluar(`(async()=>{
  const pausa=ms=>new Promise(r=>setTimeout(r,ms));
  const cuadros=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
  const rutas=['/comida','/cuerpo','/cuidado','/ayuno','/nibi','/perfil','/ajustes','/'];
  const navegacion=[];
  for(let vuelta=0;vuelta<3;vuelta++)for(const ruta of rutas){
   const inicio=performance.now();document.querySelector('nav a[href="'+ruta+'"]').click();
   const limite=inicio+10000;
   while((location.pathname||'/')!==ruta){if(performance.now()>limite)throw Error('Navegación agotada');await pausa(5);}
   await cuadros();
   while(document.querySelector('main .carga')){if(performance.now()>limite)throw Error('Carga agotada');await pausa(5);}
   if(document.querySelector('main .error-lectura'))throw Error('Lectura fallida');
   await cuadros();navegacion.push({ruta,ms:performance.now()-inicio});
  }
  const busqueda=[];
  for(let n=0;n<30;n++){
   const inicio=performance.now();
   const alimentos=await window.__TAURI_INTERNALS__.invoke('comida_buscar',{consulta:'arroz',limit:20});
   if(!alimentos.length)throw Error('Búsqueda vacía');busqueda.push(performance.now()-inicio);
  }
  return {navegacion,busqueda};
 })()`);
 assert.equal(resultado.navegacion.length,24);
 const percentil=(valores,p)=>{const ordenados=[...valores].sort((a,b)=>a-b);return Number(ordenados[Math.ceil(p*ordenados.length)-1].toFixed(1));};
 const resumen={entorno:'Linux, webview de desarrollo embebido, datos sintéticos pequeños, caché caliente',navegacion:{muestras:24,p50ms:percentil(resultado.navegacion.map(v=>v.ms),.5),p95ms:percentil(resultado.navegacion.map(v=>v.ms),.95)},busqueda:{muestras:30,p50ms:percentil(resultado.busqueda,.5),p95ms:percentil(resultado.busqueda,.95)}};
 mkdirSync('test-results/rendimiento',{recursive:true});
 writeFileSync('test-results/rendimiento/resultado.json',JSON.stringify({resumen,...resultado},null,2)+'\n');
 console.log(JSON.stringify(resumen,null,2));
} finally {i.cerrar();}
