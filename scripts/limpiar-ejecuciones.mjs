#!/usr/bin/env node
/** Solo se ejecuta en Actions: conserva la ejecución más reciente del workflow. */
import { execFileSync } from 'node:child_process';
const repositorio=process.env.GITHUB_REPOSITORY;
const actual=Number(process.env.GITHUB_RUN_ID);
const workflow=process.env.NB_WORKFLOW_ARCHIVO;
if(process.env.GITHUB_ACTIONS!=='true'||!repositorio||!Number.isSafeInteger(actual)||!['ci.yml','ediciones.yml'].includes(workflow)){
  console.error('La limpieza solo está permitida dentro del workflow autorizado de Actions.');
  process.exit(2);
}
const ejecutar=(args)=>execFileSync('gh',args,{encoding:'utf8'});
const paginas=JSON.parse(ejecutar(['api','--paginate','--slurp',`repos/${repositorio}/actions/workflows/${workflow}/runs?per_page=100`]));
const ejecuciones=paginas.flatMap(p=>p.workflow_runs);
const masReciente=Math.max(actual,...ejecuciones.map(e=>e.id));
for(const ejecucion of ejecuciones){
  // Una ejecución más antigua no elimina la actual ni interrumpe otra en curso.
  if(ejecucion.id===masReciente||ejecucion.id===actual||ejecucion.status!=='completed')continue;
  ejecutar(['api','--method','DELETE',`repos/${repositorio}/actions/runs/${ejecucion.id}`]);
}
console.log('Ejecuciones anteriores completadas: eliminadas; se conserva la última.');
