'use strict';
window.program.verify=()=>{
    let checked=0;const failure=[];
    function check(condition,name){checked++;if(!condition)failure.push(name);}
    const label=particle=>particle.map(value=>value.label).sort().join('.');
    const has=(graph,text)=>graph.node.some(value=>value.scope==='outer'&&label(value.particle)===text.split('.').sort().join('.'));
    const expected={direct:'False',negate:'False',false:'True',and:'False.True.Extra',both:'True.Extra',or:'True.False.Extra',neither:'False',chain:'False.Extra',diamond:'False',permit:'Admitted.Extra',explicit:'False.Extra',double:'True',replacement:'C.Extra',frame:'C.X',flat:'False.Extra'};
    program.example.filter(value=>value.id!=='deep').forEach(example=>{
        const graph=program.run(example,6,{limit:180});
        check(graph.node.every(value=>!program.overlap(value.particle)),example.name+' disjoint live resources');
        check(graph.node.every(value=>!(value.control||[]).some(origin=>value.particle.some(token=>token.origin.includes(origin)))),example.name+' enclosing consumption is separate from live resources');
        check(new Set(graph.event.map(value=>value.id)).size===graph.event.length,example.name+' unique events');
        check(new Set(graph.node.map(program.state)).size===graph.node.length,example.name+' canonical states');
        check(graph.event.every(event=>graph.node.some(value=>value.id===event.target)&&graph.node.some(value=>value.id===event.source)),example.name+' valid application endpoints');
        if(expected[example.id])check(has(graph,expected[example.id]),example.name+' result path');
        check(graph.event.every(event=>event.path.every(id=>graph.event.find(value=>value.id===id).round<event.round)),example.name+' only previously available evidence');
    });
    const graph=program.run(program.example[1],5,{limit:120});
    const inferred=graph.event.find(value=>value.source==='v0'&&value.rule==='entry');
    check(Boolean(inferred)&&inferred.evidence!=='v0'&&inferred.path.length===1,'Inferred entry at concrete source');
    const body=graph.node.find(value=>value.id===inferred?.target);
    check(body?.parent==='v0'&&label(body.particle)==='True','Remainder comes from source, not successor');
    const direct=graph.event.find(value=>value.rule==='entry'&&value.path.length===0);
    check(Boolean(direct)&&graph.node.find(value=>value.id===direct.target).particle.length===0,'Direct nested match consumes all matched concepts');
    const diamond=program.run(program.example[8],5,{limit:180});
    check(diamond.event.filter(value=>value.source==='v0'&&value.rule==='entry').length===1,'Equivalent evidence does not duplicate source application');
    const endpoint=diamond.event.find(value=>value.source==='v0'&&value.rule==='entry').evidence;
    check(diamond.event.filter(value=>value.target===endpoint).length>=2,'Shared evidence state retains incoming alternative derivations');
    const symmetric=program.run(program.example[4],5,{limit:180});
    check(symmetric.event.filter(value=>value.source==='v0'&&value.rule==='entry').length===1,'Repeated abstract labels have one ownership binding');
    for(const id of ['ambiguous','shortcut']){
        const result=program.run(program.example.find(value=>value.id===id),6,{limit:500});
        for(const outcome of ['Red','Blue'])check(result.node.some(value=>value.scope==='outer'&&value.particle.some(token=>token.label===outcome)),id+' reaches '+outcome);
    }
    const cycle={id:'cycle',input:['A'],pattern:['Unavailable'],body:[],rule:[{id:'ab',input:['A'],output:['B']},{id:'ba',input:['B'],output:['A']}]};
    const cyclic=program.run(cycle,5,{limit:100});
    check(cyclic.node.every(value=>new Set(value.particle.flatMap(token=>token.origin)).size===1),'Cycle creates no independent resource');
    check(cyclic.node.every(value=>new Set(value.particle.map(token=>token.label+'/'+token.origin.join(','))).size===value.particle.length),'Repeated descriptions coalesce by label and ownership');
    const closure=program.run(cycle,20,{limit:100});
    check(closure.settled&&closure.node.length<10,'Finite cycle settles into shared states');
    check(closure.event.some(value=>value.target==='v0'),'Cycle retains a back edge to the original state');
    const initial=program.initial({input:['A','B']}).node[0];
    check(program.state(initial)===program.state({...initial,particle:[...initial.particle].reverse()}),'State order is irrelevant');
    check(program.state(initial)===program.state(program.initial({input:['B','A']}).node[0]),'Initial occurrence naming is canonical');
    check(program.state(initial)!==program.state({...initial,context:'another'}),'Distinct coherence context is not erased');
    check(program.state(initial)!==program.state({...initial,caller:'another'}),'Continuation affects state identity');
    check(program.state(program.initial({input:['A']}).node[0])!==program.state(program.initial({input:['A','A']}).node[0]),'Independent multiplicity affects state identity');
    const repeat={input:['A'],pattern:['Unavailable'],body:[],rule:[{id:'loop',input:['A'],output:['A']}]};
    const loop=program.run(repeat,10,{limit:1});
    check(loop.settled&&loop.node.length===1&&loop.event.length===1,'Self-loop reuses one state and settles');
    const blocked={...repeat,rule:[{id:'grow',input:['A'],output:['B']},...repeat.rule]};
    const constrained=program.run(blocked,3,{limit:1});
    check(constrained.limited&&constrained.event.some(value=>value.rule==='loop'),'Budget still permits edges to existing states');
    const parallel={...repeat,rule:[{id:'first',input:['A'],output:['B']},{id:'second',input:['A'],output:['B']}]};
    const shared=program.run(parallel,5);
    check(shared.node.length===2&&shared.event.length===2&&shared.event[0].target===shared.event[1].target,'Different rules share one result without losing incoming events');
    const changed=program.advance(loop,blocked,{limit:10});
    check(changed.event.some(value=>value.rule==='grow'),'Changed rule environment can reopen a settled graph');
    const conjunction=program.run(program.example.find(value=>value.id==='explicit'),20,{limit:200});
    const entry=conjunction.event.find(value=>value.source==='v0'&&value.rule==='entry');
    const input=conjunction.node.find(value=>value.id===entry.target);
    const output=conjunction.event.find(value=>value.source===input.id&&value.rule==='mixed'&&value.path.length===0);
    check(label(conjunction.node.find(value=>value.id===entry.evidence).particle)==='And.Boolean.Boolean.Extra','And entry uses a complete two-Boolean evidence state');
    check(label(input.particle)==='Extra.False.True'&&input.scope==='body','And enters its body with the concrete True and False');
    check(Boolean(output)&&label(conjunction.node.find(value=>value.id===output.target).particle)==='Extra.False'&&conjunction.node.find(value=>value.id===output.target).scope==='outer','Concrete mixed body returns False with Extra');
    check(conjunction.settled,'Explicit And reference reaches a finite fixed point');
    check(conjunction.node.every(value=>program.binding(['Boolean','Boolean','Boolean'],value.particle).length===0),'Two And operands never supply three independent Boolean bindings');
    check(conjunction.node.some(value=>program.binding(['Boolean','Boolean'],value.particle).length>0),'And retains a valid two-Boolean binding');
    check(conjunction.node.every(value=>value.particle.filter(token=>['True','False','Boolean'].includes(token.label)).length<=2),'No conjunction state contains more than two Boolean operands');
    check(conjunction.node.every(value=>!program.overlap(value.particle)),'Every conjunction state has disjoint live resources');
    check(has(conjunction,'Boolean.Extra'),'Aggregate inference consumes And and both operands while preserving Extra');
    check(conjunction.event.filter(value=>value.rule==='true'&&value.path.length===0).every(event=>{
        const source=conjunction.node.find(value=>value.id===event.source),target=conjunction.node.find(value=>value.id===event.target);
        return event.selected.some(id=>source.particle.some(token=>token.id===id&&token.label==='True'))&&event.selected.every(id=>!target.particle.some(token=>token.id===id));
    }),'Every direct True to Boolean application consumes its selected True');
    const alias=[{id:'a',label:'A',origin:['x']},{id:'b',label:'B',origin:['x']}];
    check(program.binding(['A','B'],alias).length===0,'Two descriptions cannot fill two consuming slots');
    check(program.binding(['A','B'],[{...alias[0]},{...alias[1],origin:['y']}]).length===1,'Independent resources can fill two slots');
    check(program.projection([{label:'C',origin:['x']}],alias.slice(0,1)).length===1,'Inferred binding projects to concrete resource');
    check(program.projection([{label:'C',origin:['x']}],[{label:'Bundle',origin:['x','y']}])===null,'Projection cannot silently split an indivisible current resource');
    for(const id of ['reuse','missing','overlap','cycle']){
        const result=program.run(program.example.find(value=>value.id===id),5,{limit:100});
        check(!result.event.some(value=>value.rule==='entry'),id+' cannot invent a whole witness');
    }
    const example=program.example[1],paused=program.run(example,4,{limit:3});
    check(paused.limited&&paused.node.length===3,'Budget pauses');
    let resumed=paused;
    for(let index=0;index<8;index++)resumed=program.advance(resumed,example,{limit:120});
    const complete=program.run(example,8,{limit:120});
    const signature=result=>result.event.map(value=>value.source+'/'+value.rule+'/'+value.ownership.map(origin=>origin.join(',')).join('|')).sort();
    check(JSON.stringify(signature({...resumed,event:resumed.event.slice(0,complete.event.length)}))===JSON.stringify(signature(complete)),'Resume preserves the unpaused application prefix and continues');
    const directExample=program.example[0],finished=program.run(directExample,5);
    check(program.advance(finished,directExample).event.length===finished.event.length,'Rechecking finished graph creates no events');
    const deep=program.example.find(value=>value.id==='deep');
    const reversed={...program.example[8],rule:[...program.example[8].rule].reverse()};
    check(program.run(reversed,5,{limit:180}).event.filter(value=>value.source==='v0'&&value.rule==='entry').length===1,'Rule order does not duplicate equivalent evidence');
    const replacement=program.run(program.example.find(value=>value.id==='replacement'),10);
    check(has(replacement,'C.Extra')&&!has(replacement,'A.C.Extra'),'Inferred flat output replaces its concrete witness');
    const literal=program.run(program.example.find(value=>value.id==='flat'),10);
    check(has(literal,'False.Extra')&&!has(literal,'False.True.Extra'),'Literal Not result does not retain a spent True');
    const frame=program.run(program.example.find(value=>value.id==='frame'),10);
    const application=frame.event.find(value=>value.source==='v0'&&value.rule==='entry');
    check(Boolean(application)&&label(frame.node.find(value=>value.id===application.target).particle)==='C.X','Concrete-source frame survives unrelated evidence computation');
    check(application.justification.some(value=>label(frame.node.find(node=>node.id===value.version).particle)==='B.Y'),'Frame projection is exercised through evidence containing unrelated Y');
    const automatic=program.run(deep,25,{limit:80});
    check(automatic.settled&&has(automatic,'False'),'Eighteen-step inference computes and settles within eighty states');
    let cursor='v0',length=0;
    for(const rule of deep.rule){
        const event=automatic.event.find(value=>value.source===cursor&&value.rule===rule.id&&value.path.length===0);
        if(!event)break;
        cursor=event.target;length++;
    }
    check(length===18&&label(automatic.node.find(value=>value.id===cursor).particle)==='Boolean.Not','Actual graph retains the complete eighteen-step direct evidence chain');
    check(program.state({...initial,control:['x','y']})===program.state({...initial,control:['y','x']}),'Continuation consumption is canonical');
    check(program.state({...initial,control:['x']})!==program.state({...initial,control:['y']}),'Different continuation consumption remains distinct');
    const original=program.initial(program.example[1]);
    const snapshot=JSON.stringify(original);
    program.advance(original,program.example[1]);
    check(JSON.stringify(original)===snapshot,'An application does not mutate its historical source graph');
    const discarded={input:['Not','True','Extra'],pattern:['Not','Boolean'],rule:[{id:'boolean',input:['True'],output:['Boolean']}],body:[{id:'discard',input:['True'],output:[]}]};
    check(has(program.run(discarded,10),'Extra'),'Zero-output body return preserves unrelated resources');
    const constant={input:['Not','True','Extra'],pattern:['Not','True'],rule:[],body:[{id:'constant',input:[],output:['False']}]};
    check(has(program.run(constant,10),'False.Extra'),'Nullary body return can use enclosing consumption');
    const creation={input:[],pattern:['Unavailable'],rule:[{id:'create',input:[],output:['A']}],body:[]};
    try{program.run(creation,1);check(false,'Unsupported fresh allocation must be diagnosed');}catch(error){check(error instanceof RangeError&&error.message.includes('Fresh resource allocation'),'Fresh allocation reports the reference boundary instead of silently failing consumption');}
    const words=length=>Array.from({length:2**length},(_,mask)=>Array.from({length},(_,index)=>mask&(1<<index)?'A':'B'));
    for(let size=0;size<=4;size++)for(const text of words(size))for(let count=1;count<=3;count++)for(const pattern of words(count)){
        const particle=text.map((label,index)=>({label,id:'x'+index,origin:['x'+index]}));
        const actual=program.binding(pattern,particle).map(value=>value.map(item=>item.id).sort().join(',')).sort(),expected=[];
        for(let mask=0;mask<2**size;mask++){
            const subset=particle.filter((value,index)=>mask&(1<<index));
            if(subset.length===count&&label(subset)===pattern.slice().sort().join('.'))expected.push(subset.map(value=>value.id).sort().join(','));
        }
        check(JSON.stringify(actual)===JSON.stringify(expected.sort()),'Exhaustive binding '+text.join('.')+' / '+pattern.join('.'));
    }
    return {checked,failure};
};
