'use strict';
window.kernel.value=(()=>{
    const cache=new WeakMap();
    function environment(state,capture){
        if(!cache.has(state))cache.set(state,new Map());
        const index=cache.get(state);
        if(!index.has(capture))index.set(capture,kernel.state.canonical({world:[{frame:capture,particle:[]}],frame:state.frame}).key);
        return index.get(capture);
    }
    const order=value=>value.slice().sort((left,right)=>JSON.stringify(left).localeCompare(JSON.stringify(right)));
    function structure(rule){
        const term=value=>typeof value==='string'?value:{rule:structure(value.rule)};
        return {input:order(rule.input.map(particle=>order(particle.map(term)))),output:order(rule.output.map(value=>({particle:order((value.particle||[]).map(term)),...(value.body?{body:order(value.body.map(structure))}:{})}))),...(rule.negative===undefined?{}:{negative:order(rule.negative.map(particle=>order(particle.map(term))))})};
    }
    function compile(program){
        const scope={},code=new Map(),index=new Map(),name=new Map();
        const text=JSON.stringify(program);let prefix='§';
        while(text.includes(prefix))prefix+='§';
        function value(term){
            if(typeof term==='string')return term;
            const canonical=structure(term.rule),key=JSON.stringify(canonical);
            if(index.has(key))return index.get(key);
            const label=prefix+index.size;index.set(key,label);name.set(label,term.rule.name||'Rule '+index.size);
            code.set(label,rule({...canonical,name:name.get(label)},'value/'+(index.size-1)));
            return label;
        }
        function rule(item,id){
            return {...item,id,name:item.name||id,input:item.input.map(particle=>particle.map(value)),...(item.negative===undefined?{}:{negative:item.negative.map(particle=>particle.map(value))}),output:item.output.map((output,position)=>{
                const result={...output,particle:(output.particle||[]).map(value)};
                if(!output.body)return result;
                const body=id+'/'+position;declare(output.body,body);return {...result,body};
            })};
        }
        function declare(item,id){scope[id]=item.map((item,index)=>rule(item,id+'/'+index));}
        declare(program.rule,'root');
        const initial=program.initial.map(particle=>particle.map(term=>{const label=value(term);return code.has(label)?{label,capture:0}:label;}));
        return {scope,code,name,initial};
    }
    const pattern=(input,capture,code)=>input.map(particle=>particle.map(label=>code.has(label)?{label,capture}:label));
    return {compile,structure,pattern,environment};
})();
