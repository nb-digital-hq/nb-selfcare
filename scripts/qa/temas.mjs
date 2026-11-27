#!/usr/bin/env node
/** Comprueba los 24 temas mediante el selector real del escritorio. */
import {verificarAislamiento} from './aislamiento.mjs';
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {Inspector,esperar} from './inspector.mjs';
import {TEMAS} from '../../src/lib/ui/temas.ts';
import {es} from '../../src/lib/i18n/es.ts';
verificarAislamiento();
const i=await Inspector.conectar();
async function hasta(expresion){for(let n=0;n<100;n++){if(await i.evaluar(expresion))return;await esperar(100);}throw new Error('La apariencia no se actualizó');}
async function elegir(tema){await hasta(`document.querySelectorAll('button.tema').length===24&&!document.querySelector('button.tema:disabled')`);await i.evaluar(`(()=>{const b=[...document.querySelectorAll('button.tema')].find(e=>e.textContent.trim()===${JSON.stringify(es.temas[tema.id])});if(!b)throw new Error('Tema ausente');b.click();})()`);await hasta(`getComputedStyle(document.documentElement).getPropertyValue('--m3c-surface').trim()===${JSON.stringify(tema.colores.fondo)}`);assert.equal(await i.evaluar('document.documentElement.dataset.papel'),String(tema.papel));}

try{
 await i.evaluar(`document.querySelector('nav a[href="/ajustes"]').click()`);await esperar(400);
 for(const tema of TEMAS){await elegir(tema);console.log(`Correcto: ${tema.id}`);if(tema.id==='nxtpaper_eink_light'){const r=await i.enviar('Page.snapshotRect',{x:0,y:0,width:1280,height:820,coordinateSystem:'Viewport'});mkdirSync('test-results/recorrido',{recursive:true});writeFileSync('test-results/recorrido/papel-es.png',Buffer.from(r.dataURL.split(',')[1],'base64'));}}
 await elegir(TEMAS[0]);
 assert.deepEqual(await i.errores(),[]);
 console.log('Los 24 temas se aplican y el Modo Papel queda identificado correctamente.');
}finally{i.cerrar();}
