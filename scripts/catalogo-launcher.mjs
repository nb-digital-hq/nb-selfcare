#!/usr/bin/env node
/** Catálogo editorial de SelfCare; genera archivos locales y nunca publica. */
import {createHash} from 'node:crypto';
import {copyFileSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
export function formatos(version){
 if(!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version))throw Error('Versión estable inválida');
 return [
  ['tar','tar-gz',`NB.SelfCare_${version}_amd64.tar.gz`,`NB.SelfCare_${version}_amd64.tar.gz`],
  ['appimage','appimage',`NB SelfCare_${version}_amd64.AppImage`,`NB.SelfCare_${version}_amd64.AppImage`],
  ['deb','deb',`NB SelfCare_${version}_amd64.deb`,`NB.SelfCare_${version}_amd64.deb`],
  ['rpm','rpm',`NB SelfCare-${version}-1.x86_64.rpm`,`NB.SelfCare-${version}-1.x86_64.rpm`]
 ].map(([carpeta,formato,origen,archivo])=>({carpeta,formato,origen,archivo,id:`selfcare-linux-x64-${carpeta==='appimage'?'appimage':carpeta}`}));
}
export function crearCatalogo({version,emitido,clavePublica,artefactos}){
 if(!Number.isSafeInteger(emitido)||emitido<0||!Number.isSafeInteger(emitido+14*86400)||!clavePublica)throw Error('Metadatos inválidos');
 return {schema:1,owner:'nb.selfcare',repository:'nb-digital-hq/nb-selfcare',version,issued_at:emitido,expires_at:emitido+14*86400,
 artifacts:formatos(version).map(f=>{
  const a=artefactos[f.archivo];
  if(!a||!Number.isSafeInteger(a.bytes)||a.bytes<=0||!/^[a-f0-9]{64}$/.test(a.sha256)||!a.signature)throw Error(`Artefacto incompleto: ${f.archivo}`);
  return {id:`${f.id}-v${version}`,app:'nb.selfcare',version,repository:'nb-digital-hq/nb-selfcare',tag:`v${version}`,filename:f.archivo,platform:'linux',architecture:'x86_64',format:f.formato,bytes:a.bytes,sha256:a.sha256,signature:a.signature,public_key:clavePublica,...(f.carpeta==='tar'?{managed_layout:'selfcare_linux_v1'}:{})};
 })};
}
/** El actualizador independiente consume los mismos bytes y firma del tar administrable. */
export function crearManifiesto(catalogo){
 const tar=catalogo.artifacts.find(a=>a.format==='tar-gz');
 if(!tar)throw Error('Falta el tar firmado');
 return {version:catalogo.version,notes:'',pub_date:new Date(catalogo.issued_at*1000).toISOString(),platforms:{'linux-x86_64':{signature:tar.signature,url:`https://github.com/nb-digital-hq/nb-selfcare/releases/download/v${catalogo.version}/${tar.filename}`}}};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 const [bundle,salida,fecha]=process.argv.slice(2);
 if(!bundle||!salida)throw Error('Uso: node scripts/catalogo-launcher.mjs <bundle> <salida> [unix-seconds]');
 if(resolve(bundle)===resolve(salida))throw Error('La salida debe ser distinta del bundle');
 const config=JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json',import.meta.url),'utf8'));
 mkdirSync(salida,{recursive:true});const artefactos={};
 for(const f of formatos(config.version)){
  const ruta=join(bundle,f.carpeta,f.origen),datos=readFileSync(ruta);
  const signature=readFileSync(`${ruta}.sig`,'utf8').trim();
  artefactos[f.archivo]={bytes:datos.length,sha256:createHash('sha256').update(datos).digest('hex'),signature};
  copyFileSync(ruta,join(salida,f.archivo));copyFileSync(`${ruta}.sig`,join(salida,`${f.archivo}.sig`));
 }
 const catalogo=crearCatalogo({version:config.version,emitido:fecha===undefined?Math.floor(Date.now()/1000):Number(fecha),clavePublica:config.plugins.updater.pubkey,artefactos});
 writeFileSync(join(salida,'launcher-catalog.json'),JSON.stringify(catalogo,null,2)+'\n');
 writeFileSync(join(salida,'latest.json'),JSON.stringify(crearManifiesto(catalogo),null,2)+'\n');
 console.log('Catálogo local generado; falta firmarlo y verificarlo. No se ha publicado.');
}
