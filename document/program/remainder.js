'use strict';
window.remainder=(()=>{
    function merge(particle){
        return [...new Map(particle.flat().map(value=>[value.id,value])).values()];
    }
    function apply(input,binding,output){
        const selected=binding.flat();
        if(binding.some(value=>new Set(value).size!==value.length))return {allowed:false,reason:'One local occurrence cannot fill two consuming positions in the same coherence.'};
        if(binding.length!==input.length||!binding.every((value,index)=>value.every(id=>input[index].some(token=>token.id===id))))return {allowed:false,reason:'Every binding must belong to its assigned input.'};
        const residual=merge(input).filter(value=>!selected.includes(value.id));
        return {allowed:true,residual,output:output.map(value=>[...value,...residual])};
    }
    const token=(label,id)=>({label,id});
    const example=[
        {name:'Different leftovers',input:[[token('A','a'),token('X','x')],[token('B','b'),token('Y','y')]],binding:[['a'],['b']],output:['C','D'],text:'Combine X and Y once, then share that same remainder with each output coherence.'},
        {name:'One shared inherited X',input:[[token('A','a'),token('X','x')],[token('B','b'),token('X','x')]],binding:[['a'],['b']],output:['C','D'],text:'Both inputs reference the same X occurrence. The remainder contains one X, visible in both outputs.'},
        {name:'Two independently introduced X occurrences',input:[[token('A','a'),token('X','x1')],[token('B','b'),token('X','x2')]],binding:[['a'],['b']],output:['C','D'],text:'Equal labels do not erase independent multiplicity. The remainder contains X.X.'},
        {name:'A consumed shared occurrence stays consumed',input:[[token('A','a'),token('X','x')],[token('B','b'),token('X','x')]],binding:[['a','x'],['b']],output:['C','D'],text:'The rule consumes X through the first input. Its alias in the second input cannot restore it to this event’s outputs. Unselected coherences are unchanged.'},
        {name:'Independent branch computation',input:[[token('A','a'),token('X','x')],[token('B','b')]],binding:[['a'],['b']],output:['C','D'],branch:['Y','Z'],text:'Both coherences inherit X. The left computes X → Y; the right computes X → Z. The results have distinct production identities, so reunion retains independently usable Y and Z.'},
        {name:'Equal independently computed results',input:[[token('A','a'),token('X','x')],[token('B','b')]],binding:[['a'],['b']],output:['C','D'],branch:['Y','Y'],text:'Both coherences compute X → Y independently. The two production events are distinct, so reunion retains Y.Y even though their labels agree.'},
        {name:'Two equal explicit outputs',input:[[token('A','a'),token('X','x')],[token('B','b')]],binding:[['a'],['b']],output:['C','C'],text:'Two explicit output positions remain two coherence occurrences. Their equal state content can share storage; their shared X is still one inherited occurrence.'}
    ];
    function evaluate(example){
        const output=example.output.map((label,index)=>[token(label,'output'+index)]);
        const first=apply(example.input,example.binding,output);
        const evolution=example.branch?first.output.map((particle,index)=>apply([particle],[['x']],[[{...token(example.branch[index],'branch'+index),cause:['x']}]]).output[0]):first.output;
        const second=apply(evolution,output.map(value=>value.map(token=>token.id)),[[token('E','result')]]);
        return {first,evolution,second};
    }
    function verify(){
        let checked=0;const failure=[];
        const expected=[['X','Y'],['X'],['X','X'],[],['X'],['X'],['X']];
        example.forEach((value,index)=>{
            const result=evaluate(value);checked++;
            if(JSON.stringify(result.first.residual.map(token=>token.label))!==JSON.stringify(expected[index]))failure.push(value.name+' remainder');
            checked++;if(JSON.stringify(result.second.output[0].map(token=>token.label))!==JSON.stringify(['E',...(value.branch||expected[index])]))failure.push(value.name+' reunion');
            checked++;if(result.first.output.length!==2)failure.push(value.name+' output multiplicity');
        });
        checked++;if(apply([[token('X','x')]],[['x','x']],[[]]).allowed)failure.push('Repeated local consumption');
        checked++;if(!apply([[token('X','x')],[token('X','x')]],[['x'],['x']],[[token('Y','left')],[token('Y','right')]]).allowed)failure.push('Distinct coherences may explicitly consume their inherited references');
        checked++;if(apply([[token('A','a')]],[['missing']],[[]]).allowed)failure.push('Missing binding');
        const independent=evaluate(example[4]);checked++;
        if(!apply(independent.second.output,[[ 'result','branch0','branch1' ]],[[]]).allowed)failure.push('Independent branch results can be consumed together');
        return {checked,failure};
    }
    return {merge,apply,example,evaluate,verify};
})();
(()=>{
    const find=id=>document.getElementById(id);
    const show=particle=>particle.map(token=>token.label+'@'+token.id).join(' . ')||'∅';
    function render(){
        const example=remainder.example[Number(find('remainder-example').value)],result=remainder.evaluate(example);
        find('remainder-question').textContent=example.text;
        find('remainder-input').textContent=example.input.map((value,index)=>'Input '+(index+1)+': '+show(value)+'\nConsumes: '+example.binding[index].join(', ')).join('\n\n');
        find('remainder-output').textContent='Shared remainder: '+show(result.first.residual)+'\n\n'+result.first.output.map((value,index)=>'Output '+(index+1)+': '+show(value)).join('\n')+(example.branch?'\n\nAfter independent branch computation:\n'+result.evolution.map(show).join('\n'):'')+'\n\nReunion after consuming output labels:\n'+show(result.second.output[0]);
    }
    remainder.example.forEach((value,index)=>{const option=document.createElement('option');option.value=index;option.textContent=value.name;find('remainder-example').append(option);});
    find('remainder-example').onchange=render;render();
})();
