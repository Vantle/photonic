'use strict';
globalThis.kernel={};
globalThis.kernel.state=(()=>{
    function* permutation(value){
        if(!value.length){yield [];return;}
        for(let index=0;index<value.length;index++)for(const tail of permutation(value.filter((_,position)=>position!==index)))yield [value[index],...tail];
    }
    function partition(value,key){
        const group=new Map();
        value.forEach(index=>{
            const signature=key(index);
            group.set(signature,[...(group.get(signature)||[]),index]);
        });
        return [...group].sort(([left],[right])=>left.localeCompare(right)).map(([,value])=>value);
    }
    function* ordering(group,position=0,selected=[]){
        if(position===group.length){yield selected;return;}
        for(const choice of permutation(group[position]))yield* ordering(group,position+1,[...selected,...choice]);
    }
    function signature(value,frame){
        const chain=[];
        for(let cursor=value.frame;cursor!==null;cursor=frame[cursor].parent)chain.unshift([frame[cursor].scope,frame[cursor].held.map(token=>token.label).sort()]);
        return JSON.stringify([chain,value.particle.map(token=>token.label).sort()]);
    }
    function reachable(value){
        const selected=new Set(),pending=[0,...value.world.map(world=>world.frame)];
        for(const world of value.world)for(const token of world.particle)if(token.capture!==undefined)pending.push(token.capture);
        for(let position=0;position<pending.length;position++){
            const index=pending[position];
            if(index===null||selected.has(index))continue;
            selected.add(index);
            const frame=value.frame[index];
            pending.push(frame.parent,frame.lexical);
            for(const token of frame.held)if(token.capture!==undefined)pending.push(token.capture);
        }
        return [...selected];
    }
    function* arrangement(value,world,retained){
        const captured=value.world.some(world=>world.particle.some(token=>token.capture!==undefined))||retained.some(index=>value.frame[index].held.some(token=>token.capture!==undefined));
        if(!captured){
            const selected=new Set([0]),order=[0];
            function include(index){
                if(index===null||selected.has(index))return;
                include(value.frame[index].parent);
                include(value.frame[index].lexical);
                selected.add(index);order.push(index);
            }
            world.forEach(index=>include(value.world[index].frame));
            yield order;return;
        }
        const group=partition(retained.filter(index=>index!==0),index=>JSON.stringify([
            signature({frame:index,particle:[]},value.frame),
            world.flatMap((source,position)=>value.world[source].frame===index?[position]:[]),
            world.flatMap((source,position)=>value.world[source].particle.filter(token=>token.capture===index).map(token=>JSON.stringify([position,token.label]))).sort()
        ]));
        for(const order of ordering(group))yield [0,...order];
    }
    function canonical(value){
        const group=partition(value.world.map((_,index)=>index),index=>signature(value.world[index],value.frame));
        const retained=reachable(value);
        let best=null;
        for(const order of ordering(group))for(const ordered of arrangement(value,order,retained)){
            const frame=new Map(ordered.map((index,position)=>[index,position]));
            const incidence=new Map();
            function record(token,place){
                const current=incidence.get(token.id)||{label:token.label,capture:token.capture,place:[]};
                current.place.push(place);incidence.set(token.id,current);
            }
            order.forEach((index,position)=>value.world[index].particle.forEach(token=>record(token,'w'+position)));
            ordered.forEach((index,position)=>value.frame[index].held.forEach(token=>record(token,'f'+position)));
            const identity=value=>JSON.stringify([value.label,value.place,...(value.capture===undefined?[]:[frame.get(value.capture)])]);
            const resource=[...incidence].sort(([,left],[,right])=>identity(left).localeCompare(identity(right)));
            const rename=new Map(resource.map(([id],index)=>[id,'r'+index]));
            const particle=value=>value.map(token=>({id:rename.get(token.id),label:token.label,...(token.capture===undefined?{}:{capture:frame.get(token.capture)})})).sort((left,right)=>left.id.localeCompare(right.id));
            const result={world:order.map(index=>({frame:frame.get(value.world[index].frame),particle:particle(value.world[index].particle)})),frame:ordered.map(index=>({scope:value.frame[index].scope,parent:value.frame[index].parent===null?null:frame.get(value.frame[index].parent),lexical:value.frame[index].lexical===null?null:frame.get(value.frame[index].lexical),held:particle(value.frame[index].held)}))};
            const key=JSON.stringify(result);
            if(best&&best.key<=key)continue;
            best={key,value:result,world:new Map(order.map((index,position)=>[index,position])),frame,resource:rename};
        }
        return best;
    }
    function initial(particle){
        let resource=0;
        return canonical({world:particle.map(value=>({frame:0,particle:value.map(value=>({id:'initial'+resource++,label:typeof value==='string'?value:value.label,...(typeof value==='string'||value.capture===undefined?{}:{capture:value.capture})}))})),frame:[{scope:'root',parent:null,lexical:null,held:[]}]}).value;
    }
    const show=(value,display=label=>label)=>value.world.map(world=>'['+(world.particle.map(token=>display(token.label)+(token.capture===undefined?'':'@f'+token.capture)).sort().join('.')||'∅')+']@'+value.frame[world.frame].scope).sort().join(' ')||'∅';
    return {canonical,initial,show};
})();
