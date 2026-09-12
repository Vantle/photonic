'use strict';
window.kernel.flow=(()=>{
    const unique=value=>[...new Set(value)].sort();
    const cell=(world,token)=>'w'+world+'/'+token.id;
    const held=(frame,token)=>'f'+frame+'/'+token.id;
    function identity(value){
        const flow={};
        value.world.forEach((world,index)=>world.particle.forEach(token=>flow[cell(index,token)]=[cell(index,token)]));
        value.frame.forEach((frame,index)=>frame.held.forEach(token=>flow[held(index,token)]=[held(index,token)]));
        return {flow,context:value.world.map((_,index)=>[index]),frame:value.frame.map((_,index)=>index)};
    }
    function compose(view,event){
        return {
            flow:Object.fromEntries(Object.entries(event.flow).map(([key,value])=>[key,unique(value.flatMap(key=>view.flow[key]||[]))])),
            context:event.context.map(value=>[...new Set(value.flatMap(index=>view.context[index]))].sort((left,right)=>left-right)),
            frame:event.frame.map(index=>index===null?null:view.frame[index])
        };
    }
    function project(source,view,selection,frame){
        const footprint=unique(selection.flatMap(value=>value.token.flatMap(token=>view.flow[cell(value.world,token)])));
        if(footprint.some(key=>!key.startsWith('w')))return null;
        const world=[...new Set([...selection.flatMap(value=>view.context[value.world]),...footprint.map(key=>Number(key.slice(1,key.indexOf('/'))))])].sort((left,right)=>left-right);
        if(world.some(index=>source.world[index].frame!==frame))return null;
        const exact=[];
        for(const position of selection)for(const token of position.token){
            const basis=view.flow[cell(position.world,token)];
            if(basis.length!==1||!basis[0].startsWith('w'))continue;
            const [place,id]=basis[0].split('/');
            if(source.world[Number(place.slice(1))].particle.some(value=>value.id===id&&value.label===token.label&&(token.capture===undefined?value.capture===undefined:value.capture===view.frame[token.capture])))exact.push(basis[0]);
        }
        return {world,footprint,exact:unique(exact)};
    }
    function apply(source,frame,owner,rule,binding,limit={},definition){
        const node={world:[],frame:source.frame.map(value=>({...value,held:[...value.held]}))};
        const flow={},context=[],mapping=source.frame.map((_,index)=>index);
        source.frame.forEach((value,index)=>value.held.forEach(token=>flow[held(index,token)]=[held(index,token)]));
        if(definition?.closure){
            const {state,view,capture}=definition.closure,imported=new Map(),resource=new Map();
            state.frame.forEach((item,index)=>{
                for(const token of item.held){
                    const basis=view.flow[held(index,token)];
                    if(basis.length!==1)continue;
                    if(token.capture!==undefined&&view.frame[token.capture]===null)continue;
                    const [place,id]=basis[0].split('/'),position=Number(place.slice(1));
                    const original=(place[0]==='w'?source.world[position].particle:source.frame[position].held).find(value=>value.id===id);
                    const capture=token.capture===undefined?undefined:view.frame[token.capture];
                    if(original&&original.label===token.label&&original.capture===capture)resource.set(token.id,id);
                }
            });
            function include(index){
                if(index===null)return null;
                if(view.frame[index]!==null)return view.frame[index];
                if(imported.has(index))return imported.get(index);
                const position=node.frame.length;imported.set(index,position);
                node.frame.push(null);mapping.push(null);
                const original=state.frame[index];
                const item={scope:original.scope,parent:include(original.parent),lexical:include(original.lexical),held:[]};
                node.frame[position]=item;
                for(const token of original.held){
                    if(!resource.has(token.id))resource.set(token.id,'capture'+token.id);
                    const copied={...token,id:resource.get(token.id),...(token.capture===undefined?{}:{capture:include(token.capture)})};
                    item.held.push(copied);flow[held(position,copied)]=view.flow[held(index,token)];
                }
                return position;
            }
            owner=include(capture);
        }
        const returning=owner===frame&&frame!==0;
        const parent=returning?source.frame[frame].parent:frame;
        source.world.forEach((value,index)=>{
            if(binding.world.includes(index))return;
            const target=node.world.length;
            node.world.push(value);context.push([index]);
            value.particle.forEach(token=>flow[cell(target,token)]=[cell(index,token)]);
        });
        const footprint=binding.footprint;
        const consumed=returning?source.frame[frame].held.map(token=>held(frame,token)):[];
        const basis=unique([...footprint,...consumed]);
        function remainder(selected){
            const removed=new Set(selected.map(key=>key.slice(key.indexOf('/')+1))),result=new Map();
            for(const index of binding.world)for(const token of source.world[index].particle){
                if(removed.has(token.id))continue;
                const previous=result.get(token.id)||{token,basis:[]};
                previous.basis.push(cell(index,token));result.set(token.id,previous);
            }
            return [...result.values()];
        }
        rule.output.forEach((output,position)=>{
            const nested=Boolean(output.body);
            let target=parent;
            if(nested){
                target=node.frame.length;
                const reserve=new Map();
                for(const key of unique([...binding.exact,...consumed])){
                    const [place,id]=key.split('/'),index=Number(place.slice(1));
                    const token=(place[0]==='w'?source.world[index].particle:source.frame[index].held).find(value=>value.id===id);
                    const previous=reserve.get(id)||{token,basis:[]};previous.basis.push(key);reserve.set(id,previous);
                }
                node.frame.push({scope:output.body,parent,lexical:owner,held:[...reserve.values()].map(value=>value.token)});mapping.push(null);
                for(const value of reserve.values())flow[held(target,value.token)]=unique(value.basis);
            }
            const index=node.world.length,particle=[];
            for(const value of remainder(nested?binding.exact:footprint)){particle.push(value.token);flow[cell(index,value.token)]=value.basis;}
            for(const [offset,label] of (output.particle||[]).entries()){
                const token={id:'new'+position+'x'+offset,label,...(definition?.code.has(label)?{capture:owner}:{})};particle.push(token);flow[cell(index,token)]=basis;
            }
            node.world.push({frame:target,particle});context.push(binding.world);
        });
        const retained=new Set();
        function retain(index){
            if(index===null||retained.has(index))return;
            retained.add(index);retain(node.frame[index].parent);retain(node.frame[index].lexical);
            node.frame[index].held.forEach(token=>{if(token.capture!==undefined)retain(token.capture);});
        }
        retain(0);
        node.world.forEach(value=>{retain(value.frame);value.particle.forEach(token=>{if(token.capture!==undefined)retain(token.capture);});});
        if(node.world.length>(limit.world??Infinity)||node.world.reduce((sum,value)=>sum+value.particle.length,0)+[...retained].reduce((sum,index)=>sum+node.frame[index].held.length,0)>(limit.cell??Infinity)||retained.size>(limit.frame??Infinity))return null;
        const result=kernel.state.canonical(node),renamed={};
        for(const [key,value] of Object.entries(flow)){
            const [place,id]=key.split('/'),index=Number(place.slice(1));
            const destination=place[0]==='w'?result.world.get(index):result.frame.get(index);
            if(destination===undefined)continue;
            renamed[place[0]+destination+'/'+result.resource.get(id)]=unique(value);
        }
        return {state:result.value,key:result.key,flow:renamed,context:[...result.world].sort(([,left],[,right])=>left-right).map(([index])=>context[index]),frame:[...result.frame].sort(([,left],[,right])=>left-right).map(([index])=>mapping[index])};
    }
    return {identity,compose,project,apply,cell,held};
})();
