'use strict';
window.negation=(()=>{
    const rule=(head,positive=[],negative=[])=>({head,positive,negative});
    const example=[
        {name:'A later fact defeats a default',rule:[rule('P',[],['Q']),rule('R',['P'])],later:[rule('Q')],text:'P depends on absent(Q), and R depends on P. Adding Q defeats that support for both P and R.'},
        {name:'Mutual negative dependency',rule:[rule('P',[],['Q']),rule('Q',[],['P'])],later:[],text:'There are two stable interpretations. Neither is silently selected or pooled with the other.'},
        {name:'Self-negating dependency',rule:[rule('P',[],['P'])],later:[],text:'This finite program has no stable interpretation. The dependency remains represented; the runtime need not crash or erase the program.'},
        {name:'Independent support survives',rule:[rule('P'),rule('P',[],['Q']),rule('R',['P'])],later:[rule('Q')],text:'Q defeats the default justification, but the independent fact P still supports P and R.'},
        {name:'A positive cycle is not a fact',rule:[rule('P',['Q']),rule('Q',['P'])],later:[],text:'The positive loop has no grounding fact. Ordinary inductive rule semantics establishes neither atom.'},
        {name:'Explicit contrary evidence',rule:[rule('P'),rule('OppositeP')],later:[],text:'P and OppositeP are independently asserted labels in this experiment. Both are retained; unrelated claims are not invented. OppositeP is not an absence test or ASP strong negation.'}
    ];
    const atoms=rule=>[...new Set(rule.flatMap(value=>[value.head,...value.positive,...value.negative]))].sort();
    function consequence(rule,assumption){
        const reduct=rule.filter(value=>!value.negative.some(atom=>assumption.includes(atom))),answer=new Set();
        for(let changed=true;changed;){changed=false;for(const value of reduct)if(value.positive.every(atom=>answer.has(atom))&&!answer.has(value.head)){answer.add(value.head);changed=true;}}
        return [...answer].sort();
    }
    function evaluate(rule){
        const atom=atoms(rule),stable=[];
        if(atom.length>16)throw new RangeError('This exhaustive comparison supports at most sixteen atoms; this is a laboratory limit, not a language restriction.');
        for(let mask=0;mask<2**atom.length;mask++){
            const candidate=atom.filter((value,index)=>mask&(1<<index));
            if(JSON.stringify(consequence(rule,candidate))===JSON.stringify(candidate))stable.push(candidate);
        }
        let lower=[],upper=atom;
        for(;;){
            const next=consequence(rule,upper),bound=consequence(rule,next);
            if(JSON.stringify(next)===JSON.stringify(lower)&&JSON.stringify(bound)===JSON.stringify(upper))break;
            lower=next;upper=bound;
        }
        return {stable,true:lower,false:atom.filter(value=>!upper.includes(value)),undefined:upper.filter(value=>!lower.includes(value))};
    }
    function verify(){
        const failure=[];let checked=0;
        const expected=[{stable:[['P','R']],true:['P','R'],false:['Q'],undefined:[]},{stable:[['P'],['Q']],true:[],false:[],undefined:['P','Q']},{stable:[],true:[],false:[],undefined:['P']},{stable:[['P','R']],true:['P','R'],false:['Q'],undefined:[]},{stable:[[]],true:[],false:['P','Q'],undefined:[]},{stable:[['OppositeP','P']],true:['OppositeP','P'],false:[],undefined:[]}];
        example.forEach((value,index)=>{checked++;if(JSON.stringify(evaluate(value.rule))!==JSON.stringify(expected[index]))failure.push(value.name);});
        checked++;if(JSON.stringify(evaluate([...example[0].rule,...example[0].later]).true)!==JSON.stringify(['Q']))failure.push('Default retraction');
        checked++;if(JSON.stringify(evaluate([...example[3].rule,...example[3].later]).true)!==JSON.stringify(['P','Q','R']))failure.push('Independent support');
        checked++;
        try{evaluate(Array.from({length:17},(_,index)=>rule('Atom'+index)));failure.push('Exhaustive comparison budget');}catch(error){if(!(error instanceof RangeError))failure.push('Exhaustive comparison budget error');}
        return {checked,failure};
    }
    return {example,consequence,evaluate,verify};
})();
(()=>{
    const find=id=>document.getElementById(id),show=value=>value.join(', ')||'∅';
    function render(){
        const example=negation.example[Number(find('negation-example').value)],rule=find('negation-later').checked?[...example.rule,...example.later]:example.rule,result=negation.evaluate(rule);
        find('negation-question').textContent=example.text;find('negation-later').disabled=!example.later.length;
        find('negation-program').textContent=rule.map(value=>value.head+' ← '+([...value.positive,...value.negative.map(atom=>'absent('+atom+')')].join(' and ')||'fact')).join('\n');
        find('negation-model').textContent=result.stable.length?result.stable.map(value=>'{ '+show(value)+' }').join('\n'):'No stable interpretation';
        find('negation-status').textContent='True: '+show(result.true)+'\nFalse: '+show(result.false)+'\nUndefined: '+show(result.undefined);
    }
    negation.example.forEach((value,index)=>{const option=document.createElement('option');option.value=index;option.textContent=value.name;find('negation-example').append(option);});
    find('negation-example').onchange=()=>{find('negation-later').checked=false;render();};find('negation-later').onchange=render;render();
})();
