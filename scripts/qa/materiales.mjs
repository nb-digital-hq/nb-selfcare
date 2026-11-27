#!/usr/bin/env node
/** Capturas y vídeo del webview real, exclusivamente con el perfil sintético de QA. */
import {execFileSync} from 'node:child_process';
import {verificarAislamiento} from './aislamiento.mjs';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync} from 'node:fs';
import {Inspector,esperar} from './inspector.mjs';
verificarAislamiento();
const i=await Inspector.conectar();
const rutas=[['today','/'],['food','/comida'],['body','/cuerpo'],['care','/cuidado'],['fasting','/ayuno'],['nibi','/nibi'],['profile','/perfil'],['settings','/ajustes']];
// Como en el resto de la suite: inglés en la raíz y español en `es/`, en WebP.
function webp(destino,png){execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','png_pipe','-i','-','-quality','82',destino],{input:png});}
async function hasta(expresion){for(let n=0;n<100;n++){if(await i.evaluar(expresion))return;await esperar(100);}throw new Error('No terminó la navegación de captura');}
async function navegar(ruta){await i.evaluar(`document.querySelector('nav a[href="${ruta}"]').click()`);await hasta(`(location.pathname||'/')===${JSON.stringify(ruta)}`);await hasta(`!document.querySelector('main .carga')`);await i.evaluar(`document.querySelector('main').scrollTop=0`);await esperar(350);}
async function idioma(codigo){if(!await i.evaluar(`!!document.querySelector('nav a[href="/ajustes"]')`))return;await i.evaluar(`document.querySelector('nav a[href="/ajustes"]').click()`);await esperar(400);if(!await i.evaluar(`!!document.getElementById('ajuste-apariencia')`))return;await i.evaluar(`document.getElementById('ajuste-apariencia').click()`);await hasta(`document.querySelector('.idiomas')`);await i.evaluar(`[...document.querySelectorAll('.idiomas button')].find(b=>b.textContent.trim()===${JSON.stringify({es:'Español',en:'English',pt:'Português'}[codigo])}).click()`);await hasta(`document.documentElement.lang===${JSON.stringify(codigo==='pt'?'pt-BR':codigo)}`);await esperar(200);}
async function fotograma(){const r=await i.enviar('Page.snapshotRect',{x:0,y:0,width:1280,height:820,coordinateSystem:'Viewport'});return Buffer.from(r.dataURL.split(',')[1],'base64');}
try{
 // Se niega a capturar un perfil ajeno al de demostración que crea el recorrido.
 await navegar('/perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);assert.equal(await i.evaluar(`document.querySelector('main input')?.value`),'Demostración');
 for(const codigo of ['en','es']){await idioma(codigo);const carpeta=codigo==='en'?'docs/assets/screenshots':'docs/assets/screenshots/es';mkdirSync(carpeta,{recursive:true});for(const [nombre,ruta] of rutas){await navegar(ruta);const png=await fotograma();webp(`${carpeta}/${nombre}.webp`,png);if(nombre==='today')webp(`${carpeta}/demo-poster.webp`,png);}console.log(`Capturas: ${codigo}, ocho destinos`);}
 mkdirSync('docs/assets/video',{recursive:true});
 for(const [codigo,archivo] of [['en','nb-selfcare-demo.mp4'],['es','nb-selfcare-demo-es.mp4']]){
 await idioma(codigo);
 const ffmpeg=spawn('ffmpeg',['-hide_banner','-loglevel','error','-f','image2pipe','-vcodec','png','-r','2','-i','-','-vf','scale=1280:820:flags=lanczos','-c:v','libx264','-pix_fmt','yuv420p','-r','24','-movflags','+faststart','-y',`docs/assets/video/${archivo}`],{stdio:['pipe','inherit','inherit']});
 const terminado=new Promise((resolve,reject)=>{ffmpeg.once('error',reject);ffmpeg.once('exit',codigo=>codigo===0?resolve():reject(new Error(`ffmpeg terminó con ${codigo}`)));});
 for(const [,ruta] of rutas){await navegar(ruta);for(let n=0;n<6;n++){if(!ffmpeg.stdin.write(await fotograma()))await new Promise(resolve=>ffmpeg.stdin.once('drain',resolve));await esperar(350);}}
 ffmpeg.stdin.end();await terminado;
 }
 await idioma('es');
 await navegar('/');console.log('Vídeo de los ocho destinos generado desde la aplicación real.');
}finally{i.cerrar();}
