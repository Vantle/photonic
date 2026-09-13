'use strict';
globalThis.kernel.capture=(()=>{
    const frame=(held=[])=>({scope:'body',parent:0,lexical:0,held});
    const root=()=>({scope:'root',parent:null,lexical:null,held:[]});
    const token=(id,label,capture)=>({id,label,...(capture===undefined?{}:{capture})});
    function* permutation(value){
        if(!value.length){yield [];return;}
        for(let index=0;index<value.length;index++)for(const rest of permutation(value.filter((_,position)=>index!==position)))yield [value[index],...rest];
    }
    function verify(){
        let checked=0;const failure=[];
        const check=(condition,name)=>{checked++;if(!condition)failure.push(name);};
        const graph={world:[{frame:0,particle:[token('a','Rule',1),token('b','Rule',2)]}],frame:[root(),frame([token('c','Other',2)]),frame([token('d','Other',1)]),frame([token('dead','Dead',3)])]};
        const expected=kernel.state.canonical(graph);
        check(expected.value.frame.length===3,'Capture cycles retain both reachable frames and prune the unrelated frame');
        check(expected.value.world[0].particle.every(token=>token.capture!==undefined),'Canonical closure occurrences retain capture metadata');
        const sample=[
            graph,
            {world:[{frame:1,particle:[token('a','Rule',2)]},{frame:2,particle:[token('b','Rule',1)]}],frame:[root(),frame([token('held','Value')]),frame([token('held','Value')])]},
            {world:[],frame:[{...root(),held:[token('rootheld','Rule',1)]},frame([token('self','Self',1)])]},
            {world:[{frame:0,particle:[token('r','Rule',1)]}],frame:[root(),frame([token('l','Rule',2),token('m','Rule',3)]),frame([token('x','X')]),frame([token('y','Y')])]},
            {world:[{frame:0,particle:[token('r','Rule',1)]}],frame:[root(),frame([token('l','Rule',2),token('m','Rule',3)]),frame([token('x','X',3)]),frame([token('y','X',2)])]}
        ];
        sample.forEach((source,index)=>{
            const key=kernel.state.canonical(source).key;
            for(const tail of permutation(source.frame.map((_,index)=>index).slice(1))){
                const order=[0,...tail],rename=new Map(order.map((index,position)=>[index,position]));
                const replace=value=>({...value,id:'renamed/'+value.id,...(value.capture===undefined?{}:{capture:rename.get(value.capture)})});
                const changed={world:source.world.map(world=>({frame:rename.get(world.frame),particle:world.particle.map(replace).reverse()})).reverse(),frame:order.map(index=>({...source.frame[index],parent:source.frame[index].parent===null?null:rename.get(source.frame[index].parent),lexical:source.frame[index].lexical===null?null:rename.get(source.frame[index].lexical),held:source.frame[index].held.map(replace).reverse()}))};
                check(kernel.state.canonical(changed).key===key,'Capture graph '+index+' preserves identity under frame order '+order.join(','));
            }
        });
        const different={world:[{frame:0,particle:[token('a','Rule',1),token('b','Rule',2)]}],frame:[root(),frame([token('x','X')]),frame([token('y','Y')])]};
        const shared={...different,world:[{frame:0,particle:[token('a','Rule',1),token('b','Rule',1)]}]};
        check(kernel.state.canonical(different).key!==kernel.state.canonical(shared).key,'Distinct capture environments do not collapse into one shared capture');
        const initial=kernel.state.initial([[{label:'Rule',capture:0},'A']]);
        check(initial.world[0].particle.some(token=>token.label==='Rule'&&token.capture===0),'Initial rule descriptor retains its root capture');
        check(kernel.state.show(initial).includes('Rule@f0'),'Closure display includes its capture frame');
        check(kernel.state.canonical(expected.value).key===expected.key,'Capture canonicalization is idempotent');
        const captured={world:[{frame:0,particle:[token('closure','R',2)]}],frame:[root(),{scope:'outer',parent:0,lexical:0,held:[token('shared','Seed')]},{scope:'inner',parent:0,lexical:1,held:[token('shared','Seed')]}]};
        for(const mapped of [false,true]){
            const source={world:[{frame:0,particle:[mapped?token('previous','Previous',1):token('seed','Seed')]}],frame:mapped?[root(),{scope:'outer',parent:0,lexical:0,held:[token('original','Seed')]}]:[root()]};
            const basis=mapped?'f1/original':'w0/seed';
            const view={frame:[0,mapped?1:null,null],flow:{'f1/shared':[basis],'f2/shared':[basis]}};
            const result=kernel.flow.apply(source,0,null,{output:[{particle:['R']}]},{world:[0],footprint:[],exact:[]},{},{code:new Map([['R',{}]]),closure:{state:captured,view,capture:2}});
            const produced=result?.state.world.flatMap(world=>world.particle).find(token=>token.label==='R');
            const name=mapped?'Import alongside an existing captured frame':'Import a wholly new capture graph';
            check(Boolean(produced),name+' produces the requested closure');
            check(Boolean(produced)&&kernel.value.environment(captured,2)===kernel.value.environment(result.state,produced.capture),name+' preserves the complete lexical environment');
            const held=result?.state.frame.filter(frame=>frame.scope==='outer'||frame.scope==='inner').flatMap(frame=>frame.held)||[];
            check(held.length===2&&new Set(held.map(token=>token.id)).size===1,name+' retains one introduction shared across held occurrences');
            const support=result?Object.entries(result.flow).filter(([key])=>key.startsWith('f')).map(([,value])=>value):[];
            check(support.length===2&&support.every(value=>value.length===1&&value[0]===basis),name+' preserves source support for both held occurrences');
        }
        for(const reserved of [false,true])for(const transformed of [false,true])for(const code of [false,true]){
            const original=token('original',code?'Code':'A',code?0:undefined);
            const source={world:[{frame:0,particle:reserved?[]:[original]}],frame:[root()]};
            if(reserved)source.frame[0].held.push(original);
            const copied=token('witness',transformed?'Changed':original.label,code?0:undefined);
            const captured={world:[],frame:[source.frame[0],frame([copied])]};
            const basis=(reserved?'f':'w')+'0/original';
            const view={frame:[0,null],flow:{'f1/witness':[basis],...(reserved?{'f0/original':[basis]}:{})}};
            const result=kernel.flow.apply(source,0,null,{output:[{particle:['Produced']}]},{world:[0],footprint:[],exact:[]},{},{code:new Map([['Produced',{}]]),closure:{state:captured,view,capture:1}});
            const imported=result.state.frame.find(frame=>frame.scope==='body').held[0];
            const retained=reserved?result.state.frame[0].held[0]:result.state.world[0].particle.find(value=>value.label===original.label);
            check((imported.id===retained.id)===!transformed,'Imported held identity preserves exact '+(reserved?'held':'live')+' '+(code?'code':'atom')+' source; transformed='+transformed);
        }
        return {checked,failure};
    }
    return {verify};
})();
