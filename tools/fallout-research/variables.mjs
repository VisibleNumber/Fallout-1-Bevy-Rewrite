// Text .GAM defaults. Ordered rows define numeric variable indices.
export function parseVariables(text,section='GAME_GLOBAL_VARS:'){
 const lines=text.split(/\r?\n/),start=lines.findIndex(s=>s.startsWith(section));if(start<0)throw Error('Missing variable section '+section);const values={},definitions=[];
 for(let line=start+1;line<lines.length;line++){
  const original=lines[line];if(!original.trim()||original.startsWith('//'))continue;
  const body=original.split(';')[0];if(!body.trim())throw Error('Ambiguous variable declaration at line '+(line+1));const equals=body.indexOf('=');
  const match=equals<0?null:body.slice(equals+1).match(/^\s*([+-]?\d+)/);if(equals>=0&&!match)throw Error('Invalid variable value at line '+(line+1));const value=match?Number(match[1]):0;if(!Number.isInteger(value)||value< -2147483648||value>2147483647)throw Error('Variable value out of range');
  const index=definitions.length,name=(equals<0?body:body.slice(0,equals)).replace(/:\s*$/,'').trim();values[index]=value;definitions.push({index,name,value,line:line+1});
 }return {values,definitions};
}
