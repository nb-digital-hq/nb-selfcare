import { describe,expect,it } from 'vitest';
import { licenciaAdmitida } from './licencias.mjs';
const permitidas=new Set(['MIT','Apache-2.0','BSD-3-Clause','Apache-2.0 WITH LLVM-exception']);
describe('expresiones de licencia admitidas',()=>{
 it('una alternativa permisiva permite elegir esa licencia',()=>{expect(licenciaAdmitida('MIT OR GPL-3.0-only',permitidas)).toBe(true);expect(licenciaAdmitida('MIT/Apache-2.0',permitidas)).toBe(true);});
 it('una conjunción incompatible nunca pasa por contener MIT',()=>{expect(licenciaAdmitida('MIT AND GPL-3.0-only',permitidas)).toBe(false);expect(licenciaAdmitida('(MIT OR Apache-2.0) AND GPL-3.0-only',permitidas)).toBe(false);});
 it('respeta precedencia, paréntesis y excepciones específicas',()=>{expect(licenciaAdmitida('(MIT AND BSD-3-Clause) OR GPL-3.0-only',permitidas)).toBe(true);expect(licenciaAdmitida('Apache-2.0 WITH LLVM-exception',permitidas)).toBe(true);expect(licenciaAdmitida('MIT WITH Unknown-exception',permitidas)).toBe(false);});
 it('rechaza expresiones vacías, incompletas y desconocidas',()=>{for(const e of ['', 'MIT AND', '(MIT', 'MIT OR OR Apache-2.0', 'SIN DECLARAR', 'MIT + GPL-3.0-only'])expect(licenciaAdmitida(e,permitidas)).toBe(false);});
});
