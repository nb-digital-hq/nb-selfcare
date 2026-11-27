#!/usr/bin/env node
/** El runtime se entrega sin intérpretes ni invocaciones a Python. */
import { readdirSync,readFileSync } from 'node:fs';
import { join,resolve,relative } from 'node:path';
const raiz=resolve(import.meta.dirname,'..');
function recorrer(dir){return readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?recorrer(join(dir,e.name)):e.name.endsWith('.rs')?[join(dir,e.name)]:[]);}
const problemas=[];
for(const archivo of recorrer(join(raiz,'src-tauri/src'))){const texto=readFileSync(archivo,'utf8');if(/(?:Command::new|\.arg)\(\s*"(?:python(?:3(?:\.\d+)?)?|pypy\d*)"/.test(texto)||/\b(?:pyo3|rustpython)::/.test(texto))problemas.push(relative(raiz,archivo));}
if(problemas.length){console.error(`Intérprete prohibido en runtime: ${problemas.join(', ')}`);process.exit(1);}
console.log('Runtime: sin intérprete Python ni invocaciones a Python.');
