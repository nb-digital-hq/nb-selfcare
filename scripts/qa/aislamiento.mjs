/** Se comprueba el proceso que escucha el inspector, no el entorno del cliente de prueba. */
import {execFileSync} from 'node:child_process';
import {readFileSync,realpathSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {resolve,sep} from 'node:path';
export function verificarAislamiento(){
 const puerto=Number(process.env.NB_QA_INSPECTOR_PORT??9333);
 if(!Number.isInteger(puerto)||puerto<1024||puerto>65535)throw new Error('Puerto de QA inválido');
 const conexiones=execFileSync('ss',['-ltnp',`( sport = :${puerto} )`],{encoding:'utf8'});
 const pid=conexiones.match(/pid=(\d+)/)?.[1];
 if(!pid)throw new Error('No se encontró el proceso del inspector de QA');
 const entorno=readFileSync(`/proc/${pid}/environ`,'utf8').split('\0');
 const carpeta=entorno.find(e=>e.startsWith('NB_SELFCARE_TEST_DIR='))?.slice('NB_SELFCARE_TEST_DIR='.length);
 if(!carpeta||!realpathSync(carpeta).startsWith(resolve(tmpdir())+sep+'nb-selfcare-qa-'))throw new Error('El recorrido solo se permite con datos temporales aislados de QA');
 return realpathSync(carpeta);
}
