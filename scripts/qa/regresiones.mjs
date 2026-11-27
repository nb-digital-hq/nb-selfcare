#!/usr/bin/env node
/** Pruebas de errores reales de almacenamiento y concurrencia, solo en datos temporales. */
import assert from 'node:assert/strict';
import {DatabaseSync} from 'node:sqlite';
import {join} from 'node:path';
import {verificarAislamiento} from './aislamiento.mjs';
import {Inspector,esperar} from './inspector.mjs';
const carpeta=verificarAislamiento();const i=await Inspector.conectar();
const db=new DatabaseSync(join(carpeta,'datos','nb-selfcare.sqlite'));
let bloqueada=false;
async function hasta(expresion){for(let n=0;n<120;n++){if(await i.evaluar(expresion))return;await esperar(100);}throw new Error('No se cumplió: '+expresion);}
async function navegar(ruta,titulo){await i.evaluar(`document.querySelector('nav a[href="${ruta}"]').click()`);await hasta(`document.querySelector('h1')?.textContent.includes(${JSON.stringify(titulo)}) && !document.querySelector('main .carga')`);}
async function limpiarAvisos(){await i.evaluar(`document.querySelectorAll('.aviso .cerrar').forEach(b=>b.click())`);}
const campo=(etiqueta,valor)=>`(()=>{const f=document.querySelector('main form');const e=[...f.querySelectorAll('label')].find(l=>l.textContent.includes(${JSON.stringify(etiqueta)})).querySelector('input');e.value=${JSON.stringify(String(valor))};e.dispatchEvent(new Event('input',{bubbles:true}));})()`;
try{
 await i.instalarCapturaDeErrores();
 await navegar('/perfil','Perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 await i.evaluar(campo('Nombre','Borrador sin guardar'));
 await i.evaluar(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'datos:cambio',payload:null})`);
 await hasta(`document.querySelector('main input')?.value==='Demostración'`);
 console.log('Correcto: el cambio de datos remonta el formulario con los valores confirmados');
 await navegar('/ajustes','Ajustes');await i.evaluar(`document.getElementById('ajuste-red').click()`);await hasta(`![...document.querySelectorAll('label.check')].find(l=>l.textContent.includes('Permitir conexiones autorizadas')).querySelector('input').disabled`);await limpiarAvisos();
 db.exec('BEGIN IMMEDIATE');bloqueada=true;
 await i.evaluar(`(()=>{const e=[...document.querySelectorAll('label.check')].find(l=>l.textContent.includes('Permitir conexiones autorizadas')).querySelector('input');if(e.checked)throw new Error('La prueba exige red apagada');e.click();})()`);
 await hasta(`document.querySelector('.aviso[data-clase="error"]')`);
 assert.equal(await i.evaluar(`[...document.querySelectorAll('label.check')].find(l=>l.textContent.includes('Permitir conexiones autorizadas')).querySelector('input').checked`),false);
 db.exec('ROLLBACK');bloqueada=false;
 const politica=await i.evaluar(`window.__TAURI_INTERNALS__.invoke('politica_red')`);assert.equal(politica.general,false);
 console.log('Correcto: una escritura bloqueada no activa la red ni deja el interruptor encendido');
 await limpiarAvisos();
 await navegar('/comida','Comida');await i.evaluar(`document.getElementById('comida-catalogo').click()`);await esperar(300);
 db.exec('BEGIN IMMEDIATE');bloqueada=true;
 await i.evaluar(`(()=>{window.__scQaEscritores=Promise.all(Array.from({length:4},()=>window.__TAURI_INTERNALS__.invoke('guardar_preferencia',{clave:'escala',valor:100}))).catch(e=>{window.__scQaErrorEscritores=e.code;});return true;})()`);await esperar(200);
 await i.evaluar(`(()=>{const e=document.querySelector('textarea');e.value='150 g de arroz';e.dispatchEvent(new Event('input',{bubbles:true}));e.closest('form').requestSubmit();})()`);await esperar(150);
 await i.evaluar(`(()=>{const e=document.querySelector('textarea');e.value='100 g de pollo';e.dispatchEvent(new Event('input',{bubbles:true}));})()`);
 db.exec('ROLLBACK');bloqueada=false;await i.evaluar('window.__scQaEscritores');await esperar(500);
 assert.equal(await i.evaluar(`window.__scQaErrorEscritores??null`),null);
 assert.equal(await i.evaluar(`[...document.querySelectorAll('main button')].some(b=>b.textContent.trim()==='Confirmar y registrar')`),false);
 assert.equal(await i.evaluar(`document.querySelectorAll('.aviso[data-clase="error"]').length`),0);
 console.log('Correcto: una propuesta de comida tardía no confirma un texto que ya cambió');
 await navegar('/perfil','Perfil');await i.evaluar(`document.getElementById('perfil-objetivos')?.click()`);await esperar(250);await i.evaluar(`document.getElementById('perfil-salud').click()`);
 await i.evaluar(`(()=>{const f=document.querySelector('main form');const grupo=[...f.querySelectorAll('fieldset')].find(e=>e.querySelector('legend')?.textContent==='Aparatos de medición');for(const l of grupo.querySelectorAll('label')){if(l.textContent.includes('Presión arterial')||l.textContent.includes('Oxígeno')){const c=l.querySelector('input');if(!c.checked)c.click();}}f.requestSubmit();})()`);await esperar(300);
 await navegar('/cuidado','Cuidado');await i.evaluar(`document.getElementById('cuidado-salud').click()`);await hasta(`[...document.querySelectorAll('details summary')].some(s=>s.textContent==='Signos vitales')`);
 await i.evaluar(`(()=>{const d=[...document.querySelectorAll('details')].find(e=>e.querySelector('summary')?.textContent==='Signos vitales');d.open=true;d.querySelector('.menu-selector .gatillo').click();})()`);await esperar(200);await i.evaluar(`[...document.querySelectorAll('[role=option]')].find(o=>/ox/i.test(o.textContent)).click()`);
 await hasta(`(()=>{const d=[...document.querySelectorAll('details')].find(e=>e.querySelector('summary')?.textContent==='Signos vitales');return d.querySelector('input[type="number"]').value==='98';})()`);
 assert.deepEqual(await i.evaluar(`(()=>{const d=[...document.querySelectorAll('details')].find(e=>e.querySelector('summary')?.textContent==='Signos vitales');const e=d.querySelector('input[type="number"]');return [e.min,e.max];})()`),['50','100']);
 console.log('Correcto: aparatos configurables y límites de oxígeno coherentes');
 await i.evaluar(`document.getElementById('cuidado-tratamientos').click()`);await hasta(`[...document.querySelectorAll('.fila')].some(e=>e.textContent.includes('Tratamiento de demostración'))`);
 await i.evaluar(`(()=>{const fila=[...document.querySelectorAll('.fila')].find(e=>e.textContent.includes('Tratamiento de demostración')&&[...e.querySelectorAll('button')].some(b=>b.textContent==='Tomada'));[...fila.querySelectorAll('button')].find(b=>b.textContent==='Deshacer')?.click();})()`);
 await hasta(`[...document.querySelectorAll('.fila')].some(e=>e.textContent.includes('Tratamiento de demostración')&&e.querySelector('.detalle')?.textContent.includes('Pendiente'))`);
 const monedas=await i.evaluar(`window.__TAURI_INTERNALS__.invoke('nibi_estado').then(e=>e.monedas)`);
 await i.evaluar(`(()=>{const fila=[...document.querySelectorAll('.fila')].find(e=>e.textContent.includes('Tratamiento de demostración')&&[...e.querySelectorAll('button')].some(b=>b.textContent==='Tomada'));if(!fila)throw new Error('No hay toma de demostración');[...fila.querySelectorAll('button')].find(b=>b.textContent==='Tomada').click();})()`);await esperar(400);
 assert.equal(await i.evaluar(`window.__TAURI_INTERNALS__.invoke('nibi_estado').then(e=>e.monedas)`),monedas);
 console.log('Correcto: marcar una toma no concede monedas');
 // Registro rápido con Ctrl + K desde cualquier vista: agua por texto y cierre del diálogo.
 await navegar('/','Demostración');const aguaAntes=await i.evaluar(`[...document.querySelectorAll('.metrica')].find(m=>m.textContent.includes('Agua'))?.querySelector('strong')?.textContent`);
 await i.evaluar(`window.dispatchEvent(new KeyboardEvent('keydown',{key:'k',ctrlKey:true,bubbles:true}))`);await hasta(`document.getElementById('rapida-texto')`);
 await i.evaluar(`(()=>{const e=document.getElementById('rapida-texto');e.value='agua 300';e.dispatchEvent(new Event('input',{bubbles:true}));})()`);await esperar(150);
 await i.evaluar(`document.querySelector('.entrada-rapida form').requestSubmit()`);await hasta(`!document.querySelector('.entrada-rapida')`);
 await hasta(`[...document.querySelectorAll('.metrica')].find(m=>m.textContent.includes('Agua'))?.querySelector('strong')?.textContent!==${JSON.stringify('__')}+''&&[...document.querySelectorAll('.metrica')].find(m=>m.textContent.includes('Agua'))?.querySelector('strong')?.textContent!==${JSON.stringify(String(aguaAntes))}`);
 console.log('Correcto: el registro rápido guarda agua desde el teclado y se cierra');
 // Recetas: crear desde la comida del día, registrar una ración y conservar la sección tras el cambio de datos.
 await navegar('/comida','Comida');await i.evaluar(`document.getElementById('comida-recetas').click()`);await hasta(`document.querySelector('main form.campos input')`);
 await i.evaluar(`(()=>{const f=document.querySelector('main form.campos');const [n,r]=f.querySelectorAll('input');n.value='Receta QA';n.dispatchEvent(new Event('input',{bubbles:true}));r.value='2';r.dispatchEvent(new Event('input',{bubbles:true}));f.requestSubmit();})()`);
 await hasta(`[...document.querySelectorAll('.receta strong')].some(e=>e.textContent==='Receta QA')`);
 await i.evaluar(`[...document.querySelectorAll('.receta')].find(e=>e.querySelector('strong').textContent==='Receta QA').querySelector('button.primario').click()`);await esperar(600);
 assert.equal(await i.evaluar(`document.getElementById('comida-recetas').getAttribute('aria-selected')`),'true');
 console.log('Correcto: una receta se crea desde la comida, se registra por raciones y la vista se conserva');
 await limpiarAvisos();assert.deepEqual(await i.errores(),[]);
}finally{if(bloqueada)db.exec('ROLLBACK');db.close();i.cerrar();}
