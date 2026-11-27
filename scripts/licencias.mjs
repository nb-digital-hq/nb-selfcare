/** Evalúa AND, OR y WITH sin aceptar una rama incompatible de una conjunción. */
export function licenciaAdmitida(expresion, permitidas) {
  const normalizada=expresion.replaceAll('/', ' OR ').trim();
  if(!/^[A-Za-z0-9.+:()\s-]+$/.test(normalizada)) return false;
  const tokens=normalizada.match(/[()]|[A-Za-z0-9.+:-]+/g) ?? [];
  let posicion=0;
  function atomo(){
    if(tokens[posicion]==='('){posicion++;const valor=disyuncion();if(tokens[posicion++]!==')')throw new Error('expresión inválida');return valor;}
    let licencia=tokens[posicion++];
    if(!licencia || ['AND','OR','WITH',')'].includes(licencia))throw new Error('expresión inválida');
    if(tokens[posicion]==='WITH'){posicion++;const excepcion=tokens[posicion++];if(!excepcion || ['AND','OR','WITH','(',')'].includes(excepcion))throw new Error('excepción inválida');licencia+=` WITH ${excepcion}`;}
    return permitidas.has(licencia);
  }
  function conjuncion(){let valor=atomo();while(tokens[posicion]==='AND'){posicion++;const otro=atomo();valor=valor && otro;}return valor;}
  function disyuncion(){let valor=conjuncion();while(tokens[posicion]==='OR'){posicion++;const otro=conjuncion();valor=valor || otro;}return valor;}
  try {const valor=disyuncion();return posicion===tokens.length && valor;}catch{return false;}
}
