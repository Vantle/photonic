'use strict';
window.kernel.model=(()=>{
    const signature=view=>JSON.stringify([view.source,view.target,Object.entries(view.flow).sort(([left],[right])=>left.localeCompare(right)),view.context,view.frame]);
    function create(program,option={}){
        const compiled=kernel.value.compile(program),{scope,code,name}=compiled,node=[],event=[],view=[],clause=[],agenda=[],pending=new Map();
        const index={node:new Map(),event:new Map(),view:new Map(),clause:new Set()};
        let cursor=0;
        const cache=new Map(),outgoing=new Map(),incoming=new Map();
        let limit={state:80,world:4,cell:12,frame:10,...option},work=0;
        const append=(index,key,value)=>{if(!index.has(key))index.set(key,[]);index.get(key).push(value);};
        function* match(pattern,target,frame){
            const key=JSON.stringify([target,frame,pattern]);
            if(!cache.has(key))cache.set(key,{value:[],iterator:kernel.binding.world(pattern,node[target].state,frame),complete:false});
            const entry=cache.get(key);
            for(let position=0;;position++){
                if(position<entry.value.length){yield entry.value[position];continue;}
                if(entry.complete)return;
                const next=entry.iterator.next();
                if(next.done){entry.complete=true;return;}
                entry.value.push(next.value);yield next.value;
            }
        }
        function support(head,premise=[]){
            const rule={head,premise:[...new Set(premise)].sort()},key=JSON.stringify(rule);
            if(index.clause.has(key))return;
            index.clause.add(key);clause.push(rule);
        }
        function* single(action){yield action;}
        const enqueue=action=>agenda.push(single(action));
        function* compose(current){
            for(const step of outgoing.get(current.target)||[])yield ()=>extend(current,step);
        }
        function extend(current,step){
            const result={source:current.source,target:step.target,...kernel.flow.compose(current,step)};
            const target=intern(result);
            support(target.id,[current.id,step.id]);
        }
        function* inspect(current){
            const source=node[current.source].state,target=node[current.target].state;
            for(let frame=0;frame<source.frame.length;frame++){
                if(frame!==0&&!source.world.some(value=>value.frame===frame))continue;
                for(let owner=frame;owner!==null;owner=source.frame[owner].lexical){
                    for(const rule of scope[source.frame[owner].scope]){
                        for(let destination=0;destination<target.frame.length;destination++){
                            if(current.frame[destination]!==frame)continue;
                            for(const selection of match(kernel.value.pattern(rule.input.length?rule.input:[[]],current.frame.indexOf(owner),code),current.target,destination)){
                                const binding=kernel.flow.project(source,current,selection,frame);
                                if(binding)yield ()=>apply(current,frame,owner,rule,binding);
                            }
                        }
                    }
                }
            }
            for(let destination=0;destination<target.world.length;destination++){
                const site=target.world[destination],frame=current.frame[site.frame];
                if(frame===null)continue;
                for(const token of site.particle){
                    const rule=code.get(token.label);
                    if(!rule)continue;
                    const input=kernel.value.pattern(rule.input,token.capture,code);
                    const candidate=match(input.length?input:[[]],current.target,site.frame);
                    for(const selection of candidate){
                        if(!selection.some(value=>value.world===destination))continue;
                        const binding=kernel.flow.project(source,current,selection,frame);
                        if(!binding)continue;
                        const read=current.flow[kernel.flow.cell(destination,token)];
                        const closure={state:target,view:current,capture:token.capture};
                        const environment=kernel.value.environment(target,token.capture);
                        const identity={value:token.label,environment,read};
                        yield ()=>apply(current,frame,current.frame[token.capture],rule,{...binding,read},closure,identity);
                    }
                }
            }
        }
        function intern(value){
            const key=signature(value);
            if(index.view.has(key))return view[index.view.get(key)];
            const result={...value,id:'v'+view.length};index.view.set(key,view.length);view.push(result);append(incoming,result.target,result);
            agenda.push(inspect(result),compose(result));return result;
        }
        function state(value,key=JSON.stringify(value)){
            if(index.node.has(key))return index.node.get(key);
            const position=node.length;index.node.set(key,position);node.push({id:position,state:value});
            const current=intern({source:position,target:position,...kernel.flow.identity(value)});support(current.id);
            return position;
        }
        function apply(current,frame,owner,rule,binding,closure,identity){
            const source=current.source,key=JSON.stringify([source,frame,owner,rule.id,binding,identity]);
            let step=index.event.get(key);
            if(step===undefined){
                const result=kernel.flow.apply(node[source].state,frame,owner,rule,binding,limit,{code,closure});
                if(!result||!index.node.has(result.key)&&node.length>=limit.state){pending.set(key+'/'+current.id,()=>apply(current,frame,owner,rule,binding,closure,identity));return;}
                const target=state(result.state,result.key);
                step=event.length;index.event.set(key,step);
                const value={id:'e'+step,source,target,rule:rule.id,name:rule.name,owner,binding,flow:result.flow,context:result.context,frame:result.frame,evidence:[]};
                event.push(value);append(outgoing,source,value);
                for(const previous of incoming.get(source)||[])enqueue(()=>extend(previous,value));
                support('s'+target,[value.id]);
            }
            const value=event[step];
            if(!value.evidence.includes(current.id))value.evidence.push(current.id);
            support(value.id,['s'+source,current.id]);
        }
        state(kernel.state.initial(compiled.initial));support('s0');
        function run(budget=1000,setting){
            if(setting){limit={...limit,...setting};for(const action of pending.values())enqueue(action);pending.clear();}
            for(let count=0;count<budget&&cursor<agenda.length;count++){
                const task=agenda[cursor++],next=task.next();work++;
                if(next.done)continue;
                agenda.push(task);next.value();
            }
            if(cursor>4096){agenda.splice(0,cursor);cursor=0;}
            return api;
        }
        const api={node,event,view,clause,scope,code,name,run,get work(){return work;},get queued(){return agenda.length-cursor;},get deferred(){return pending.size;},get closed(){return cursor===agenda.length&&!pending.size;},get limit(){return {...limit};}};
        return api;
    }
    return {create};
})();
