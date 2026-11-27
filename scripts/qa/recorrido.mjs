#!/usr/bin/env node
/** Recorrido de la aplicación real con un perfil de demostración aislado. */
import {verificarAislamiento} from './aislamiento.mjs';
import assert from 'node:assert/strict';
import { mkdirSync,writeFileSync } from 'node:fs';
import { join,resolve } from 'node:path';
import { Inspector,esperar } from './inspector.mjs';

const salida=resolve('test-results/recorrido');mkdirSync(salida,{recursive:true});
verificarAislamiento();
const inspector=await Inspector.conectar();
const evaluar=e=>inspector.evaluar(e);
async function esperarHasta(expresion){const limite=Date.now()+15000;while(Date.now()<limite){if(await evaluar(expresion))return;await esperar(100);}throw new Error(`No se cumplió la condición: ${expresion}`);}
async function navegar(ruta,titulo){await evaluar(`document.querySelector('nav a[href="${ruta}"]').click()`);await esperarHasta(`(location.pathname||'/')===${JSON.stringify(ruta)}&&!!document.querySelector('h1')`);await esperarHasta(`!document.querySelector('main .carga')`);}
async function idioma(codigo){if(!await evaluar(`!!document.querySelector('nav a[href="/ajustes"]')`))return;await evaluar(`document.querySelector('nav a[href="/ajustes"]').click()`);await esperar(400);if(!await evaluar(`!!document.getElementById('ajuste-apariencia')`))return;await evaluar(`document.getElementById('ajuste-apariencia').click()`);await esperarHasta(`document.querySelector('.idiomas')`);await evaluar(`[...document.querySelectorAll('.idiomas button')].find(b=>b.textContent.trim()===${JSON.stringify({es:'Español',en:'English',pt:'Português'}[codigo])}).click()`);await esperarHasta(`document.documentElement.lang===${JSON.stringify(codigo==='pt'?'pt-BR':codigo)}`);await esperar(200);}
async function captura(nombre){const r=await inspector.enviar('Page.snapshotRect',{x:0,y:0,width:1280,height:820,coordinateSystem:'Viewport',omitDeviceScaleFactor:true});assert.ok(r.dataURL?.startsWith('data:image/png;base64,'));writeFileSync(join(salida,`${nombre}.png`),Buffer.from(r.dataURL.split(',')[1],'base64'));}
const preparacion=`
window.__scQaCampo=(formulario,etiqueta,valor)=>{const label=[...formulario.querySelectorAll('label')].find(e=>e.textContent.includes(etiqueta));if(!label)throw new Error('Campo ausente: '+etiqueta);const input=label.querySelector('input,select,textarea');input.value=String(valor);input.dispatchEvent(new Event(input.tagName==='SELECT'?'change':'input',{bubbles:true}));};
window.__scQaFormulario=texto=>{const f=[...document.querySelectorAll('main form')].find(e=>e.textContent.includes(texto));if(!f)throw new Error('Formulario ausente: '+texto);return f;};
`;
const resultados=[];
function ok(nombre){resultados.push(nombre);console.log(`Correcto: ${nombre}`);}
try{
 await esperarHasta(`document.querySelector('h1')`);
 await inspector.instalarCapturaDeErrores();
 await idioma('es');await evaluar('(()=>{'+preparacion+';return true;})()');
 const bienvenida=await evaluar(`document.querySelector('h1').textContent.includes('Un espacio')`);
 if(bienvenida){await evaluar(`(()=>{const f=document.querySelector('main form');__scQaCampo(f,'Nombre','Demostración');__scQaCampo(f,'Fecha de nacimiento','1990-01-01');f.requestSubmit();})()`);}
 // La última vista se restaura: el recorrido empieza siempre en Hoy.
 await esperarHasta(`document.querySelector('nav a[href="/"]')`);await evaluar(`document.querySelector('nav a[href="/"]').click()`);await esperarHasta(`document.querySelector('h1')?.textContent.includes('Demostración')`);
 ok('perfil creado desde el formulario y resumen cargado');
 await evaluar(`[...document.querySelectorAll('main button')].find(e=>e.textContent.includes('Escribir en mi diario')).click()`);
 await evaluar(`(()=>{const f=__scQaFormulario('Tu diario');__scQaCampo(f,'¿Cómo estuvo','Un día tranquilo de demostración.');f.requestSubmit();})()`);
 await esperarHasta(`document.querySelector('.nota')?.textContent.includes('demostración')`);ok('diario guardado y leído');await captura('hoy-es');
 await navegar('/comida','Comida');await evaluar(`document.querySelector('.cabecera-vista .primario').click()`);
 await evaluar(`(()=>{const e=document.querySelector('main input[type="search"]');e.value='arroz';e.dispatchEvent(new Event('input',{bubbles:true}));})()`);
 await esperarHasta(`document.querySelector('.alimento strong')?.textContent.toLowerCase().includes('arroz')`);
 await evaluar(`document.querySelector('.alimento').click()`);await esperarHasta(`document.querySelector('dialog[open] input[type="number"]')`);
 await evaluar(`(()=>{const f=document.querySelector('dialog[open] form');__scQaCampo(f,'Cantidad (g)',150);f.requestSubmit();})()`);
 await esperarHasta(`!document.querySelector('dialog[open]')`);await esperarHasta(`Number(document.querySelector('.metricas .metrica strong')?.textContent.replace(',','.'))>0`);ok('catálogo, cajón y registro de comida con nutrientes');
 await evaluar(`document.getElementById('comida-hoy').click()`);
 await evaluar(`(()=>{const f=__scQaFormulario('Cantidad de agua');__scQaCampo(f,'Cantidad de agua',500);f.requestSubmit();})()`);
 await esperarHasta(`document.body.innerText.includes('500 ml')`);ok('agua persistida');await captura('comida-es');
 await navegar('/cuerpo','Cuerpo');await evaluar(`document.querySelector('.cabecera-vista .primario').click()`);await evaluar(`(()=>{const f=__scQaFormulario('Peso (kg)');__scQaCampo(f,'Peso (kg)',71.2);f.requestSubmit();})()`);await esperarHasta(`document.querySelector('.tabla')?.textContent.includes('71,2')`);ok('pesaje, tendencia y tabla actualizados');await captura('cuerpo-es');
 await navegar('/cuidado','Cuidado');await evaluar(`document.getElementById('cuidado-habitos').click()`);await evaluar(`[...document.querySelectorAll('main button')].find(e=>e.textContent.trim()==='Nuevo hábito').click()`);await evaluar(`(()=>{const f=__scQaFormulario('Nuevo hábito');__scQaCampo(f,'Nombre','Paseo de demostración');f.requestSubmit();})()`);await esperarHasta(`document.body.innerText.includes('Paseo de demostración')`);
 await evaluar(`[...document.querySelectorAll('label.check')].find(e=>e.textContent.includes('Paseo de demostración')).querySelector('input').click()`);await esperar(250);await navegar('/','Hoy');await navegar('/cuidado','Cuidado');await evaluar(`document.getElementById('cuidado-habitos').click()`);await esperarHasta(`[...document.querySelectorAll('label.check')].find(e=>e.textContent.includes('Paseo de demostración'))?.querySelector('input').checked`);ok('hábito marcado y conservado al volver a la vista');
 await evaluar(`document.getElementById('cuidado-tratamientos').click()`);await evaluar(`(()=>{const d=[...document.querySelectorAll('details')].find(e=>e.querySelector('summary')?.textContent==='Tratamientos');d.querySelector('summary').click();const f=d.querySelectorAll('form')[0];__scQaCampo(f,'Nombre','Tratamiento de demostración');__scQaCampo(f,'Dosis indicada','Según indicación');f.requestSubmit();})()`);await esperarHasta(`document.body.innerText.includes('Tratamiento de demostración')`);ok('tratamiento registrado sin recompensas por toma');await captura('cuidado-es');
 await navegar('/ayuno','Ayuno');const habilitar=await evaluar(`[...document.querySelectorAll('main button')].some(e=>e.textContent==='Habilitar el registro de ayuno')`);if(habilitar){await evaluar(`[...document.querySelectorAll('main button')].find(e=>e.textContent==='Habilitar el registro de ayuno').click()`);await esperarHasta(`document.querySelector('form input[type="number"]')`);}
 await evaluar(`(()=>{const f=document.querySelector('main form');__scQaCampo(f,'Objetivo (horas)',12);f.requestSubmit();})()`);await esperarHasta(`document.querySelector('.cronometro')`);ok('inicio de ayuno con comprobación previa');await captura('ayuno-es');await evaluar(`[...document.querySelectorAll('main button')].find(e=>e.textContent==='Terminar').click()`);await esperarHasta(`!document.querySelector('.cronometro')`);ok('ayuno terminado e historial disponible');
 await navegar('/nibi','Nibi');await esperarHasta(`document.querySelector('.dragon')`);assert.equal(await evaluar(`document.querySelectorAll('.articulo').length`),32);ok('Nibi y catálogo local de 32 artículos');await captura('nibi-es');
 await navegar('/perfil','Perfil');await esperarHasta(`document.getElementById('perfil-objetivos')`);await evaluar(`document.getElementById('perfil-objetivos').click()`);await esperarHasta(`document.querySelector('main input')?.value==='Demostración'`);ok('perfil conservado y editable');await captura('perfil-es');
 await navegar('/ajustes','Ajustes');await esperarHasta(`document.querySelectorAll('button.tema').length===24`);await captura('ajustes-es');
 for(const [codigo,titulo] of [['en','Settings'],['pt','Configurações'],['es','Ajustes']]){await idioma(codigo);await esperarHasta(`document.querySelector('h1')?.textContent.startsWith(${JSON.stringify(titulo)})`);await captura(`ajustes-${codigo}`);}ok('cambio de idioma en la ventana real sin perder la vista');
 const errores=await inspector.errores();assert.deepEqual(errores,[]);ok('sin errores de JavaScript durante el recorrido');
 writeFileSync(join(salida,'resultado.json'),JSON.stringify({resultado:'correcto',comprobaciones:resultados},null,2)+'\n');
}catch(error){await captura('fallo').catch(()=>{});writeFileSync(join(salida,'resultado.json'),JSON.stringify({resultado:'fallo',comprobaciones:resultados,error:String(error),consola:await inspector.errores().catch(()=>[])},null,2)+'\n');throw error;}
finally{inspector.cerrar();}
