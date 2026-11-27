/** Teclado y ampliación del texto en el webview real, con el perfil sintético de QA. */
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector,esperar} from './inspector.mjs';
verificarAislamiento();
const i=await Inspector.conectar();
async function hasta(expresion){for(let n=0;n<100;n++){if(await i.evaluar(expresion))return;await esperar(100);}throw Error('No se confirmó el estado accesible');}
async function navegar(ruta){await i.evaluar(`document.querySelector('nav a[href="${ruta}"]').click()`);await hasta(`(location.pathname||'/')===${JSON.stringify(ruta)}&&!document.querySelector('main .carga')`);await esperar(200);}
async function escala(valor){await navegar('/ajustes');await hasta(`!!document.getElementById('ajuste-apariencia')`);await i.evaluar(`document.getElementById('ajuste-apariencia').click()`);await hasta(`!!document.querySelector('.tipografia form input[type="number"]')`);await i.evaluar(`(()=>{const e=document.querySelector('.tipografia form input[type="number"]');e.value=${valor};e.dispatchEvent(new Event('input',{bubbles:true}));e.form.requestSubmit();})()`);await hasta(`!document.querySelector('.tipografia form button').disabled`);await esperar(150);}
try{
 await i.instalarCapturaDeErrores();
 await navegar('/comida');await hasta(`!!document.getElementById('comida-hoy')`);
 await i.evaluar(`(()=>{const e=document.getElementById('comida-hoy');e.focus();e.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}));})()`);
 await hasta(`document.getElementById('comida-catalogo').getAttribute('aria-selected')==='true'`);
 assert.equal(await i.evaluar('document.activeElement.id'),'comida-catalogo');
 await i.evaluar(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}))`);
 await hasta(`document.activeElement.id==='comida-hoy'&&document.activeElement.getAttribute('aria-selected')==='true'`);
 console.log('Correcto: pestañas de tarea navegables con flechas y Home');
 await navegar('/ajustes');await i.evaluar(`(()=>{const e=document.getElementById('ajuste-apariencia');e.focus();e.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',bubbles:true}));})()`);
 await hasta(`document.activeElement.id==='ajuste-avisos'&&document.activeElement.getAttribute('aria-selected')==='true'`);
 await i.evaluar(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}))`);
 await hasta(`document.activeElement.id==='ajuste-apariencia'`);
 console.log('Correcto: navegación interna de Ajustes con teclado y foco conservado');
 await escala(150);
 const resultados=[];
 for(const ruta of ['/','/comida','/cuerpo','/cuidado','/ayuno','/nibi','/perfil','/ajustes']){
  await navegar(ruta);
  const medida=await i.evaluar(`(()=>{const e=document.querySelector('main');return {ancho:e.clientWidth,contenido:e.scrollWidth};})()`);
  resultados.push({ruta,...medida});assert.ok(medida.contenido<=medida.ancho+1,`Desbordamiento horizontal en ${ruta}`);
 }
 mkdirSync('test-results/accesibilidad',{recursive:true});writeFileSync('test-results/accesibilidad/resultado.json',JSON.stringify({escala:150,resultados},null,2)+'\n');
 console.log('Correcto: ocho destinos sin desbordamiento horizontal al ampliar texto al 150 %');
 await escala(100);
 await i.evaluar(`(()=>{const e=document.querySelector('.carcasa');e.style.width='680px';e.style.height='560px';})()`);
 await hasta(`document.querySelector('.carcasa').dataset.ancho==='compacto'`);
 assert.ok(await i.evaluar(`(()=>{const e=document.querySelector('.registro-rapido');return (e.getAttribute('aria-label')||e.title||[...e.querySelectorAll('span')].filter(s=>getComputedStyle(s).display!=='none').map(s=>s.textContent).join('')).trim();})()`),'La acción compacta necesita nombre accesible aunque oculte el texto');
 await i.evaluar(`document.querySelector('.carcasa').style.width='760px'`);
 await hasta(`document.querySelector('.carcasa').dataset.ancho==='medio'`);
 const compactos=[];
 for(const ruta of ['/','/comida','/cuerpo','/cuidado','/ayuno','/nibi','/perfil','/ajustes']){
  await navegar(ruta);
  const medida=await i.evaluar(`(()=>{const e=document.querySelector('main');return {ancho:e.clientWidth,contenido:e.scrollWidth};})()`);
  compactos.push({ruta,...medida});assert.ok(medida.contenido<=medida.ancho+1,`Desbordamiento en lienzo compacto: ${ruta}`);
 }
 writeFileSync('test-results/accesibilidad/compacto.json',JSON.stringify({lienzo:{ancho:760,alto:560},compactos},null,2)+'\n');
 const captura=await i.enviar('Page.snapshotRect',{x:0,y:0,width:760,height:560,coordinateSystem:'Viewport'});
 writeFileSync('test-results/accesibilidad/compacto.png',Buffer.from(captura.dataURL.split(',')[1],'base64'));
 console.log('Correcto: lienzo compacto de 760 × 560 y acción con nombre accesible');

 assert.deepEqual(await i.errores(),[]);
}finally{try{await i.evaluar(`(()=>{const e=document.querySelector('.carcasa');e.style.removeProperty('width');e.style.removeProperty('height');})()`);await escala(100);}finally{i.cerrar();}}
