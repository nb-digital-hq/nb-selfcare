import {describe,it,expect} from 'vitest';
import {formatos,crearCatalogo,crearManifiesto} from './catalogo-launcher.mjs';
const artefactos=Object.fromEntries(formatos('1.0.0').map(f=>[f.archivo,{bytes:123,sha256:'a'.repeat(64),signature:'firma'}]));
const entrada={version:'1.0.0',emitido:100,clavePublica:'clave',artefactos};
describe('catálogo de distribución SelfCare',()=>{
 it('solo el tar permite gestión y todos los formatos pertenecen a SelfCare',()=>{
  const c=crearCatalogo(entrada);
  expect(c.expires_at-c.issued_at).toBe(14*86400);
  expect(c.artifacts.filter(a=>a.managed_layout).map(a=>a.format)).toEqual(['tar-gz']);
  expect(c.artifacts.every(a=>a.app==='nb.selfcare'&&a.repository==='nb-digital-hq/nb-selfcare'&&!a.filename.includes(' '))).toBe(true);
  expect(crearCatalogo(entrada)).toEqual(c);
 });
 it('el actualizador independiente referencia exactamente el tar y su firma',()=>{
  const c=crearCatalogo(entrada),m=crearManifiesto(c),tar=c.artifacts.find(a=>a.format==='tar-gz');
  expect(m.platforms['linux-x86_64']).toEqual({signature:tar.signature,url:`https://github.com/nb-digital-hq/nb-selfcare/releases/download/v1.0.0/${tar.filename}`});
  expect(m.version).toBe(c.version);
 });
 it('rechaza archivos ausentes y metadatos incompletos',()=>{
  expect(()=>crearCatalogo({...entrada,artefactos:{}})).toThrow('incompleto');
  expect(()=>crearCatalogo({...entrada,emitido:NaN})).toThrow('inválidos');
 });
 it('la versión no puede introducir rutas ni ediciones preliminares',()=>{
  for(const version of ['../1.0.0','1.0.0-beta','01.0.0','1.0.0/archivo'])expect(()=>formatos(version)).toThrow();
 });
});
