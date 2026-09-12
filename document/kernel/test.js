'use strict';
window.kernel.verify=()=>{
    let checked=0;const failure=[];
    const check=(condition,name)=>{checked++;if(!condition)failure.push(name);};
    const run=(example,limit={})=>kernel.model.create(example,limit).run(12000);
    const find=(model,text)=>model.node.find(value=>kernel.state.show(value.state)===text);
    const has=(model,text)=>Boolean(find(model,text));
    const arrival=(world,label,id='token')=>({world,token:[{id,label}]});
    const gate=kernel.gate.create([['A'],['B']]);
    check([...gate.arrive(1,arrival(1,'B'))].length===0,'Later slot waits for earlier slot');
    check([...gate.arrive(1,arrival(1,'B'))].length===0,'Duplicate arrival does not fill another slot');
    check([...gate.arrive(0,arrival(0,'A'))].length===1,'Out-of-order arrivals produce one complete joint binding');
    check([...gate.arrive(0,arrival(0,'A'))].length===0,'Rediscovery does not pulse an existing binding');
    check([...gate.arrive(0,arrival(2,'A'))].length===1,'New occurrence produces a distinct application candidate');
    const repeated=kernel.gate.create([['A'],['A']]);
    [...repeated.arrive(0,arrival(0,'A'))];
    check([...repeated.arrive(1,arrival(0,'A'))].length===0,'One coherence cannot fill two input positions');
    check([...repeated.arrive(1,arrival(1,'A'))].length===1,'Distinct sibling coherences fill repeated pattern positions');
    const separate=kernel.gate.create([['A'],['B']]);
    check([...separate.arrive(1,arrival(1,'B'))].length===0,'Another joint witness has a separate gate');
    function* enumerate(pattern,state,frame,selected=[]){
        if(!pattern.length){yield selected;return;}
        for(let index=0;index<state.world.length;index++){
            const world=state.world[index];
            if(world.frame!==frame||selected.some(value=>value.world===index))continue;
            const previous=selected.findLast(value=>JSON.stringify(value.pattern)===JSON.stringify(pattern[0]));
            if(previous&&previous.world>=index)continue;
            for(const token of kernel.binding.particle(pattern[0],world.particle))yield* enumerate(pattern.slice(1),state,frame,[...selected,{world:index,token,pattern:pattern[0],position:selected.length}]);
        }
    }
    const signature=binding=>JSON.stringify(binding.map(value=>[value.world,value.token.map(token=>token.id)]));
    for(let mask=0;mask<64;mask++){
        const state={world:Array.from({length:3},(_,world)=>({frame:world===2&&mask%2?1:0,particle:['A','B'].filter((_,index)=>mask&(1<<(world*2+index))).map(label=>({label,id:world+label}))}))};
        for(const pattern of [[],[[]],[['A']],[['A'],['B']],[['A'],['A']],[['B'],['A']],[['A','B'],[]]]){
            const actual=[...kernel.binding.world(pattern,state,0)].map(signature).sort();
            const expected=[...enumerate(pattern,state,0)].map(signature).sort();
            check(JSON.stringify(actual)===JSON.stringify(expected),'Incremental gate agrees with exhaustive binding '+mask+' '+JSON.stringify(pattern));
        }
    }
    const models=kernel.example.slice(0,14).map((example,index)=>index===7?kernel.model.create(example,{cell:3,state:12}).run(2000):run(example));
    models.forEach((model,index)=>{
        check(new Set(model.node.map(value=>JSON.stringify(value.state))).size===model.node.length,'Canonical states '+index);
        check(model.node.every(value=>value.state.world.every(world=>new Set(world.particle.map(token=>token.id)).size===world.particle.length)),'No duplicate local introductions '+index);
        check(model.event.every(value=>model.node[value.source]&&model.node[value.target]),'Existing event endpoints '+index);
        check(model.view.every(value=>Object.keys(value.flow).length===model.node[value.target].state.world.reduce((sum,world)=>sum+world.particle.length,0)+model.node[value.target].state.frame.reduce((sum,frame)=>sum+frame.held.length,0)),'Complete flow maps '+index);
        if(index!==7)check(model.closed,'Finite fixture closes '+index);
    });
    check(has(models[0],'[Extra.False]@root'),'And returns concrete False');
    check(has(models[0],'[Boolean.Extra]@root'),'And aggregate consumes structural input');
    check(models[0].node.every(value=>value.state.world.every(world=>world.particle.filter(token=>['True','False','Boolean'].includes(token.label)).length<=2)),'And never accumulates extra Boolean operands');
    check(has(models[1],'[C.X.Y]@root [D.X.Y]@root')&&has(models[1],'[E.X.Y]@root'),'Broadcast and inherited reconciliation');
    check(models[2].event.some(value=>value.source===0&&kernel.state.show(models[2].node[value.target].state)==='[D.X]@root'&&value.binding.world.length===1),'Joint siblings project to one source once');
    check(!has(models[3],'[D]@root'),'Competing histories never form a joint witness');
    check(has(models[4],'[B]@root [B]@root')&&models[4].event.some(value=>value.source===0&&kernel.state.show(models[4].node[value.target].state)==='[D]@root'),'Explicit equal outputs remain two witness occurrences');
    check(has(models[5],'[D.Extra]@root')&&has(models[5],'[C.D.Extra]@root'),'Source inference and executed evidence retain distinct outcomes');
    check(has(models[6],'[D.X]@root')&&has(models[6],'[D.Y]@root')&&has(models[6],'[D.Y.Y]@root'),'Unchanged, shared-computed, and independently computed remainders differ');
    check(!models[7].closed&&models[7].deferred>0&&has(models[7],'[B]@root'),'Fresh creation consumes correctly and bounded growth stays pending');
    check(has(models[8],'[Extra.False]@root')&&models[8].node.some(value=>value.state.frame.length>=4),'Three nested expression bodies execute');
    const status=(model,text)=>kernel.support.evaluate(model).status('s'+find(model,text).id);
    check(status(models[9],'[P]@root')==='supported','Closed default is supported');
    for(const index of [9,10]){
        const example=kernel.example[index],revised=run({...example,rule:[...example.rule,...example.later]});
        check(status(revised,'[P]@root')===(index===9?'unsupported':'supported'),'Revision and alternative support '+index);
        check(status(models[index],'[P]@root')==='supported','Prior revision remains intact '+index);
    }
    check(status(models[11],'[P]@root')==='conditional','Self-negating support remains conditional');
    check(kernel.support.interpretation(models[11]).value.length===0,'Self-negation has no stable interpretation but retains its state');
    check(kernel.support.interpretation(models[12]).value.length===2,'Mutual absence retains two stable interpretations');
    check(models[13].node.length===2&&models[13].closed,'Finite cycle reuses two configurations');
    const paused=kernel.model.create(kernel.example[13],{state:1}).run(500);
    check(!paused.closed&&paused.deferred>0,'State budget defers new configuration');
    paused.run(12000,{state:80});
    check(paused.closed&&JSON.stringify(paused.node.map(value=>JSON.stringify(value.state)).sort())===JSON.stringify(models[13].node.map(value=>JSON.stringify(value.state)).sort()),'Resume reaches the unpaused fixed point');
    const step=kernel.model.create(kernel.example[11]);
    while(!step.query.length)step.run(1);
    check(!step.closed&&status(step,'[P]@root')==='conditional','Open recursive absence is not mistaken for failure');
    const early=kernel.model.create(kernel.example[9]);
    while(!early.query.length)early.run(1);
    check(!early.closed&&early.query[0].closed&&status(early,'[P]@root')==='supported','Finite label invariant closes a query without global completion');
    const root={scope:'root',parent:null,lexical:null,held:[]};
    const shared={world:[{frame:0,particle:[{id:'x',label:'X'}]},{frame:0,particle:[{id:'x',label:'X'}]}],frame:[root]};
    const independent={...shared,world:[shared.world[0],{frame:0,particle:[{id:'y',label:'X'}]}]};
    check(kernel.state.canonical(shared).key!==kernel.state.canonical(independent).key,'Canonical key preserves cross-coherence sharing');
    for(let mask=0;mask<256;mask++){
        const identity=Array.from({length:4},(_,index)=>Math.floor(mask/4**index)%4);
        if(identity[0]===identity[1]||identity[2]===identity[3])continue;
        const value={world:[0,1].map(index=>({frame:0,particle:identity.slice(index*2,index*2+2).map(id=>({id:'old'+id,label:'X'}))})),frame:[root]};
        const renamed={...value,world:value.world.slice().reverse().map(world=>({
            ...world,
            particle:world.particle.slice().reverse().map(token=>({
                ...token,id:'new'+(7-Number(token.id.slice(3)))
            }))
        }))};
        const left=kernel.state.canonical(value),right=kernel.state.canonical(renamed);
        check(left.key===right.key,'Permutation and fresh-name invariance '+mask);
        check(kernel.state.canonical(left.value).key===left.key,'Canonicalization idempotence '+mask);
    }
    const body={world:[{frame:1,particle:[{id:'x',label:'X'}]}],frame:[root,{scope:'body',parent:0,lexical:0,held:[{id:'a',label:'A'}]}]};
    const returned=kernel.flow.apply(body,1,1,{output:[{particle:['Y','Z']}]},{world:[0],footprint:['w0/x'],exact:['w0/x']},{cell:2,frame:1,world:1});
    check(Boolean(returned)&&kernel.state.show(returned.state)==='[Y.Z]@root','Discarded continuation frames do not consume the target budget');
    const literal=(name,input,output,negative)=>({name,input,output:output.map(particle=>({particle})),...(negative===undefined?{}:{negative})});
    const derived=run({initial:[['Start']],rule:[literal('Default',[['Start']],[['P']],[['Q']]),literal('Consequence',[['P']],[['Q']])]});
    check(status(derived,'[P]@root')==='conditional'&&status(derived,'[Q]@root')==='conditional','Derived negative cycle remains conditional across source projection');
    const combined={initial:[['A','X']],rule:[literal('Conditional split',[['A']],[['B'],['C']],[['Q']]),literal('Joint result',[['B'],['C']],[['D']])]};
    const before=run(combined),after=run({...combined,rule:[...combined.rule,literal('Later route',[['A']],[['Q']])]});
    check(status(before,'[D.X]@root')==='supported'&&status(after,'[D.X]@root')==='unsupported','Conditional evidence propagates through decoherence and ancestor projection');
    const allocation=run({initial:[['A','X'],['B','X']],rule:[literal('Broadcast',[['A'],['B']],[['C'],['D']]),literal('Reunion',[['C'],['D']],[['E']])]});
    check(has(allocation,'[E.X.X]@root'),'Independent initial leftovers retain multiplicity');
    const local={initial:[['Enter','Ready','Call','Payload']],rule:[
        {name:'Outer',input:[['Enter','Ready']],output:[{body:[literal('Local trap',[['Payload']],[['Bad']])]}]},
        {name:'Call',input:[['Call']],output:[{body:[literal('Finish',[['Payload']],[['Good']])]}]}
    ]};
    const lexical=run(local);
    check(lexical.node.some(value=>value.state.frame.some(frame=>frame.scope==='root/1/0'&&frame.parent!==frame.lexical)),'Lexical capture differs from the return continuation');
    check(!lexical.event.some(event=>event.name==='Local trap'&&event.binding.world.some(index=>lexical.node[event.source].state.frame[lexical.node[event.source].state.world[index].frame].scope==='root/1/0')),'Caller-local rules do not leak into a lexically defined body');
    if(kernel.dynamic){
        const dynamic=kernel.dynamic.example.map((example,index)=>kernel.model.create(example,{state:80,cell:index===7?4:9,frame:10}).run(12000));
        dynamic.forEach((model,index)=>{
            if(index!==7)check(model.closed,'Dynamic example closes '+index);
            check(new Set(model.node.map(value=>JSON.stringify(value.state))).size===model.node.length,'Dynamic state interning '+index);
            check(model.view.every(view=>Object.keys(view.flow).length===Object.keys(kernel.flow.identity(model.node[view.target].state).flow).length),'Dynamic views retain complete live and captured flow '+index);
            check(model.event.every(event=>event.binding.read===undefined||event.binding.read.every(key=>key.startsWith('w')||key.startsWith('f'))),'Explicit availability footprint '+index);
        });
        check(has(dynamic[0],'[B.Seed]@root'),'Generated code reads Seed without consuming it');
        check(dynamic[0].event.some(event=>event.source===0&&kernel.state.show(dynamic[0].node[event.target].state)==='[B.Seed]@root'&&event.binding.read.length===1),'Read footprint belongs to inferred source');
        check(has(dynamic[1],'[B]@root'),'Consumption wins when code and data share Seed');
        check(!dynamic[2].node.some(node=>node.state.world.some(world=>world.particle.some(token=>token.label==='B'))),'Competing code and data do not enable an application');
        const replacement=dynamic[3],successor=replacement.event.find(event=>event.source===0&&event.name==='Replace rule').target;
        check(replacement.event.some(event=>event.source===successor&&event.name==='A to C')&&!replacement.event.some(event=>event.source===successor&&event.name==='A to B'),'Replacement successor uses its own available rule');
        check(dynamic[4].node.every(node=>node.state.world.some(world=>world.particle.length===1&&world.particle[0].label==='A')),'Unrelated coherence cannot borrow local code');
        check(has(dynamic[5],'[C.Extra.Seed]@root'),'Dynamic decoherence preserves read-only source remainder');
        const conditional=dynamic[6],unsupported=conditional.node.filter(node=>node.state.world.some(world=>world.particle.some(token=>token.label==='P')));
        check(unsupported.length>0&&unsupported.every(node=>kernel.support.evaluate(conditional).status('s'+node.id)==='unsupported'),'Generated Q rule defeats dependent default');
        check(conditional.query.every(query=>!query.closed),'Dynamic code disables fixed-rule absence certificates');
        check(dynamic[7].node.some(node=>node.id!==0&&kernel.support.evaluate(dynamic[7]).status('s'+node.id)==='conditional'),'Conditional generated code retains conditional support');
        check(has(dynamic[8],'[Done]@root'),'Whole-rule abstraction transfers its concrete witness');
        check(has(dynamic[9],'[Done.Seed]@root'),'Nested dynamic return does not consume code read support');
        check(dynamic[10].node.some(node=>node.state.world.some(world=>world.frame===0&&world.particle.some(token=>token.label==='Done'))),'Escaped closure uses its captured local definition');
        check(dynamic[10].node.some(node=>node.state.frame.length>=3&&node.state.world.some(world=>world.particle.some(token=>token.capture!==undefined&&token.capture!==world.frame))),'Captured definition and invocation frames remain distinct');
        check(has(dynamic[11],'[B]@root')&&status(dynamic[11],'[B]@root')==='supported','Later code consumption does not invalidate historical results');
        const revised=kernel.model.create(kernel.dynamic.example[0],{state:1}).run(1000);
        revised.run(12000,{state:80});
        check(revised.closed&&JSON.stringify(revised.node.map(node=>JSON.stringify(node.state)).sort())===JSON.stringify(dynamic[0].node.map(node=>JSON.stringify(node.state)).sort()),'Dynamic capture and read dependencies survive budget resumption');
        const coded={rule:literal('First',[['A','B']],[['C']])};
        const reordered={rule:literal('Renamed',[['B','A']],[['C']])};
        const compiled=kernel.value.compile({initial:[[coded,reordered,'§0']],rule:[]});
        check(compiled.code.size===1&&compiled.initial[0][0].label===compiled.initial[0][1].label,'Rule code interning normalizes particle order and ignores display name');
        check(compiled.initial[0][0].label!=='§0','Rule value identities cannot collide with atom labels');
        const root={scope:'root',parent:null,lexical:null,held:[]};
        const local={scope:'local',parent:0,lexical:0,held:[]};
        const captured={world:[{frame:0,particle:[{id:'r',label:'Rule',capture:1}]}],frame:[root,local]};
        const global={...captured,world:[{frame:0,particle:[{id:'r',label:'Rule',capture:0}]}]};
        check(kernel.state.canonical(captured).key!==kernel.state.canonical(global).key,'Same code under different capture has distinct canonical identity');
        const loop={world:[{frame:0,particle:[{id:'r',label:'Rule',capture:1}]}],frame:[root,{...local,held:[{id:'q',label:'Rule',capture:1}]}]};
        const canonical=kernel.state.canonical(loop);
        check(canonical.value.frame.length===2&&kernel.state.canonical(canonical.value).key===canonical.key,'Capture cycle is retained without recursive expansion');
        const transformed=run({initial:[['Seed','A']],rule:[...kernel.dynamic.example[0].rule,literal('Continue',[['B']],[['C']])]});
        check(has(transformed,'[C.Seed]@root'),'Read-only availability does not enter transitive consuming flow');
    }
    const creation={name:'Create',input:[],output:[{particle:['Y']}]};
    const anchored=kernel.model.create({initial:[['X']],rule:[creation]},{cell:2}).run(100);
    check(has(anchored,'[X.Y]@root'),'Lexical zero-input rule preserves its execution-site remainder');
    const active=kernel.model.create({initial:[['X',{rule:creation}]],rule:[]},{cell:3}).run(100);
    check(active.node.some(node=>node.state.world.length===1&&node.state.world[0].particle.some(token=>token.label==='X')&&node.state.world[0].particle.some(token=>token.label==='Y')),'Local zero-input rule uses the same anchored application semantics');
    check(kernel.model.create({initial:[],rule:[creation]}).run(100).event.length===0,'Zero-input application requires an execution-site coherence');
    if(kernel.capture){const result=kernel.capture.verify();checked+=result.checked;failure.push(...result.failure);}
    return {checked,failure};
};
