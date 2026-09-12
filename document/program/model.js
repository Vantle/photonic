'use strict';
window.program = (() => {
    function binding(pattern, particle) {
        let answer = [[]];
        for (const label of pattern) {
            const next = [];
            for (const selected of answer) for (const token of particle) {
                if (token.label !== label || selected.some(value=>value.id===token.id || (value.origin||[value.id]).some(origin=>(token.origin||[token.id]).includes(origin)))) continue;
                const previous=selected.filter(value=>value.label===label).at(-1);
                if(previous&&ownership(previous)>=ownership(token))continue;
                next.push([...selected,token]);
            }
            answer=next;
        }
        return answer;
    }
    const ownership=value=>(value.origin||[value.id]).slice().sort().join(',');
    function removal(selected,particle){
        return particle.filter(value=>selected.some(view=>view.label===value.label&&ownership(view)===ownership(value)));
    }
    function projection(witness,particle){
        const origin=new Set(witness.flatMap(value=>value.origin));
        const selected=particle.filter(value=>value.origin.some(item=>origin.has(item)));
        if(selected.some(value=>value.origin.some(item=>!origin.has(item))))return null;
        if([...origin].some(item=>!selected.some(value=>value.origin.includes(item))))return null;
        return selected;
    }
    function overlap(particle){
        return particle.some((value,index)=>particle.slice(index+1).some(other=>value.origin.some(origin=>other.origin.includes(origin))));
    }
    function display(particle){
        return particle.map(value=>value.label).join('.')||'∅';
    }
    function covers(pattern,particle){return binding(pattern,particle).length>0;}
    function state(value){
        return JSON.stringify([value.context,value.caller,[...(value.control||[])].sort(),value.particle.map(token=>[token.label,[...new Set(token.origin)].sort()]).sort((left,right)=>JSON.stringify(left).localeCompare(JSON.stringify(right)))]);
    }
    function initial(example){
        const particle=example.input.slice().sort().map((label,index)=>({id:'x'+index,label,origin:['x'+index]}));
        return {node:[{id:'v0',particle,scope:'outer',context:'outer',caller:null,control:[],parent:null,depth:0}],event:[],round:0,limited:false,settled:false};
    }
    function reachable(graph,source){
        const pending=[{node:source,path:[]}],seen=new Set([source.id]);
        for(let position=0;position<pending.length;position++){
            const current=pending[position];
            for(const event of graph.event.filter(value=>value.source===current.node.id)){
                if(seen.has(event.target))continue;
                seen.add(event.target);
                pending.push({node:graph.node.find(value=>value.id===event.target),path:[...current.path,event.id]});
            }
        }
        return pending.filter(value=>value.node.context===source.context&&value.node.caller===source.caller&&JSON.stringify(value.node.control||[])===JSON.stringify(source.control||[]));
    }
    function advance(graph,example,option={}){
        const setting={visibility:'inherited',limit:80,...option};
        const node=[...graph.node],event=[...graph.event];
        const identity=new Set(event.map(value=>value.id));
        const intern=new Map(node.map(value=>[state(value),value]));
        const entry={id:'entry',input:example.pattern,output:example.output||[],nested:!example.output};
        let limited=false;
        for(const source of graph.node){
            const local=source.scope==='body'?example.body:[];
            const inherited=source.scope==='outer'||setting.visibility==='inherited'?example.rule:[];
            const available=[...inherited.map(rule=>({rule,environment:'outer',returns:false})),...local.map(rule=>({rule,environment:source.context,returns:true})),...(source.scope==='outer'?[{rule:entry,environment:'outer',returns:false}]:[])];
            const view=reachable(graph,source);
            for(const application of available){
                const rule=application.rule;
                for(const evidence of view)for(const witness of binding(rule.input,evidence.node.particle)){
                    const footprint=projection(witness,source.particle);
                    if(!footprint)continue;
                    const exact=removal(witness,source.particle);
                    const selected=rule.nested?exact:footprint;
                    const key=source.id+'/'+application.environment+'/'+rule.id+'/'+witness.map(value=>value.label+':'+ownership(value)).join('|');
                    if(identity.has(key)){
                        const index=event.findIndex(value=>value.id===key),existing=event[index];
                        if(!existing.justification.some(value=>value.version===evidence.node.id))event[index]={...existing,justification:[...existing.justification,{version:evidence.node.id,path:evidence.path}]};
                        continue;
                    }
                    const remainder=source.particle.filter(value=>!selected.some(item=>item.id===value.id));
                    let target='v'+node.length;
                    const origin=[...new Set([...footprint.flatMap(value=>value.origin),...(application.returns?source.control||[]:[])])].sort();
                    if(rule.output.length>1)throw new RangeError('Multiple-output allocation is outside this reference fragment.');
                    if(rule.output.length&&!origin.length)throw new RangeError('Fresh resource allocation without a source footprint is outside this reference fragment.');
                    const particle=rule.nested?remainder:[...rule.output.map((label,index)=>({id:target+'x'+index,label,origin})),...remainder];
                    const context=rule.nested?'body':application.returns?source.caller:source.context;
                    const scope=context==='outer'?'outer':'body';
                    const caller=rule.nested?source.context:application.returns?null:source.caller;
                    const control=rule.nested?[...new Set([...(source.control||[]),...exact.flatMap(value=>value.origin)])].sort():application.returns?[]:source.control||[];
                    const candidate={id:target,particle,scope,context,caller,control,parent:source.id,depth:source.depth+1};
                    const canonical=state(candidate),existing=intern.get(canonical);
                    if(existing)target=existing.id;
                    else{
                        if(node.length>=setting.limit){limited=true;continue;}
                        node.push(candidate);intern.set(canonical,candidate);
                    }
                    event.push({id:key,source:source.id,target,rule:rule.id,environment:application.environment,input:rule.input,output:rule.output,nested:Boolean(rule.nested),footprint:footprint.map(value=>value.id),transfer:rule.nested?footprint.filter(value=>!exact.some(item=>item.id===value.id)).map(value=>value.id):[],selected:selected.map(value=>value.id),evidence:evidence.node.id,path:evidence.path,ownership:witness.map(value=>value.origin),justification:[{version:evidence.node.id,path:evidence.path}],round:graph.round+1});
                    identity.add(key);
                }
            }
        }
        return {node,event,round:graph.round+1,limited,settled:!limited&&event.length===graph.event.length};
    }
    function run(example,round,option={}){
        let graph=initial(example);
        for(let step=0;step<round&&!graph.settled;step++)graph=advance(graph,example,option);
        return graph;
    }
    return {binding,removal,projection,overlap,display,covers,state,initial,reachable,advance,run};
})();
