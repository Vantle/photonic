'use strict';
window.coherence=(()=>{
    const example=[
        {name:'Joint siblings',initial:['A'],context:{A:'κ0',B:'κ1',C:'κ2'},event:[{id:'split',input:['A'],output:['B','C']}],witness:['B','C'],source:['B','C'],question:'B and C were jointly produced by one event. Their common ancestry does not make them incompatible.'},
        {name:'Siblings evolve independently',initial:['A'],context:{A:'κ0',B:'κ1',C:'κ2',U:'κ1',V:'κ2'},event:[{id:'split',input:['A'],output:['B','C']},{id:'left',input:['B'],output:['U']},{id:'right',input:['C'],output:['V']}],witness:['U','V'],source:['B','C'],question:'A match at U and V can license a source-level application at B and C because one joint derivation supports both.'},
        {name:'The same ancestor twice',initial:['A'],context:{A:'κ0',B:'κ1',C:'κ2'},event:[{id:'split',input:['A'],output:['B','C']}],witness:['B','C'],source:['A','A'],question:'Replacing both siblings by A repeats one source occurrence. The two-input rule cannot spend that source twice.'},
        {name:'Competing histories',initial:['X'],context:{X:'κ0',B:'κ1',C:'κ2'},event:[{id:'left',input:['X'],output:['B']},{id:'right',input:['X'],output:['C']}],witness:['B','C'],source:['B','C'],question:'B and C each exist in some history, but no compatible frontier contains both.'},
        {name:'One shared prerequisite',initial:['A','B','T'],context:{A:'κ1',B:'κ2',T:'κ3',U:'κ1',V:'κ2'},event:[{id:'left',input:['A','T'],output:['U']},{id:'right',input:['B','T'],output:['V']}],witness:['U','V'],source:['A','B'],question:'Both component derivations need T. Independent ancestry checks miss the shared prerequisite and its conflicting consumption.'},
        {name:'Independent roots',initial:['A','B'],context:{A:'κ1',B:'κ2',U:'κ1',V:'κ2'},event:[{id:'left',input:['A'],output:['U']},{id:'right',input:['B'],output:['V']}],witness:['U','V'],source:['A','B'],question:'Distinct source contexts have one joint derivation to the witness tuple. Either event order is admissible.'},
        {name:'One advanced source',initial:['A'],context:{A:'κ0',B:'κ1',C:'κ2',U:'κ1',V:'κ2'},event:[{id:'split',input:['A'],output:['B','C']},{id:'left',input:['B'],output:['U']},{id:'right',input:['C'],output:['V']}],witness:['U','V'],source:['B','V'],question:'B and V coexist after the right branch advances. Only the remaining B → U step is needed.'}
    ];
    function frontier(initial,event,done=[]){
        const pending=[{live:[...initial].sort(),done:[...done].sort()}],seen=new Set([pending[0].live.join(',')+'/'+pending[0].done.join(',')]);
        for(let index=0;index<pending.length;index++){
            const state=pending[index];
            for(const rule of event){
                if(new Set(rule.input).size!==rule.input.length||state.done.includes(rule.id)||!rule.input.every(value=>state.live.includes(value)))continue;
                const next={live:[...state.live.filter(value=>!rule.input.includes(value)),...rule.output].sort(),done:[...state.done,rule.id].sort()};
                const key=next.live.join(',')+'/'+next.done.join(',');
                if(!seen.has(key)){seen.add(key);pending.push(next);}
            }
        }
        return pending;
    }
    function ancestor(example,source,target){
        const seen=new Set([source]);
        for(let changed=true;changed;){changed=false;for(const event of example.event)if(event.input.some(value=>seen.has(value)))for(const value of event.output)if(!seen.has(value)){seen.add(value);changed=true;}}
        return seen.has(target);
    }
    function evaluate(example,source=example.source){
        if(new Set(source).size!==source.length)return {allowed:false,reason:'One source occurrence cannot fill both input positions.'};
        if(new Set(source.map(value=>example.context[value])).size!==source.length)return {allowed:false,reason:'The source tuple must retain distinct coherence contexts.'};
        if(!source.every((value,index)=>ancestor(example,value,example.witness[index])))return {allowed:false,reason:'A selected source does not lead to its assigned witness.'};
        const start=frontier(example.initial,example.event).filter(state=>source.every(value=>state.live.includes(value)));
        if(!start.length)return {allowed:false,reason:'The selected sources never coexist in a compatible frontier.'};
        for(const state of start){
            const proof=frontier(source,example.event,state.done).find(value=>example.witness.every(witness=>value.live.includes(witness)));
            if(proof)return {allowed:true,reason:'One compatible source frontier and one joint derivation support the complete witness tuple.',start:state.done,path:proof.done.filter(value=>!state.done.includes(value))};
        }
        return {allowed:false,reason:'No joint derivation produces both witnesses from the bound sources. Missing or competing prerequisites cannot be silently borrowed.'};
    }
    function verify(){
        const expected=[true,true,false,false,false,true,true],failure=[];
        example.forEach((value,index)=>{if(evaluate(value).allowed!==expected[index])failure.push(value.name);});
        if(frontier(['A'],[{id:'twice',input:['A','A'],output:['B']}]).length!==1)failure.push('Distinct consuming occurrences');
        return {checked:example.length+1,failure};
    }
    return {example,frontier,ancestor,evaluate,verify};
})();
(()=>{
    const find=id=>document.getElementById(id);
    function render(reset=false){
        const example=coherence.example[Number(find('coherence-example').value)];
        if(reset)for(const [index,id] of ['coherence-left','coherence-right'].entries()){
            find(id).replaceChildren();
            for(const value of Object.keys(example.context).filter(value=>coherence.ancestor(example,value,example.witness[index]))){const option=document.createElement('option');option.value=value;option.textContent=value+' · '+example.context[value];find(id).append(option);}
            find(id).value=example.source[index];
        }
        const source=[find('coherence-left').value,find('coherence-right').value],result=coherence.evaluate(example,source);
        find('coherence-question').textContent=example.question;
        find('coherence-net').textContent='Initial: '+example.initial.join(', ')+'\n'+example.event.map(value=>value.input.join(' + ')+' → '+value.output.join(' + ')).join('\n')+'\n\nMatched witnesses: '+example.witness.join(' + ');
        find('coherence-result').textContent=(result.allowed?'Enabled: ':'Not enabled by this model: ')+source.join(' + ');
        find('coherence-result').className=result.allowed?'good':'bad';
        find('coherence-reason').textContent=result.reason;
        find('coherence-path').textContent=result.allowed?'Source frontier after: '+(result.start.join(', ')||'initialization')+'\nJoint evidence steps: '+(result.path.join(', ')||'identity'):'No joint witness for this source tuple.';
    }
    coherence.example.forEach((value,index)=>{const option=document.createElement('option');option.value=index;option.textContent=value.name;find('coherence-example').append(option);});
    find('coherence-example').onchange=()=>render(true);find('coherence-left').onchange=()=>render();find('coherence-right').onchange=()=>render();render(true);
})();
