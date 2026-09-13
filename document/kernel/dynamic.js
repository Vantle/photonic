'use strict';
globalThis.kernel.dynamic=(()=>{
    const literal=(name,input,output)=>({name,input,output:output.map(particle=>({particle}))});
    const value=rule=>({rule});
    const rule=value(literal('A to B',[['A']],[['B']]));
    const replacement=value(literal('A to C',[['A']],[['C']]));
    const generated=literal('Generate rule',[['Seed']],[[rule]]);
    const example=[
        {name:'Generated rule reads its concrete source',initial:[['Seed','A']],rule:[generated],text:'Actual execution produces a rule and then B. Source inference instead consumes A and preserves Seed, which supplied the code. No deduced rule is copied into Seed.B.'},
        {name:'One source supplies rule and operand',initial:[['Seed']],rule:[literal('Generate both',[['Seed']],[[rule,'A']])],text:'Seed jointly supplies a rule and A. Inferred application consumes Seed through A, even though Seed also supports the rule read. The result is B.'},
        {name:'Competing code and data cannot interact',initial:[['Seed']],rule:[generated,literal('Alternative operand',[['Seed']],[['A']])],text:'Rule and A occur in competing configurations. Their historical existence does not enable B.'},
        {name:'A meta rule replaces a whole rule value',initial:[[rule,'A']],rule:[literal('Replace rule',[[rule]],[[replacement]])],text:'The original rule can compute B. Replacing its whole value creates a successor with A → C, which can compute C. No global definition is mutated.'},
        {name:'Rule activation respects coherence locality',initial:[[rule,'A'],['A']],rule:[],text:'Only the coherence containing the rule can apply it. The independent A remains unchanged.'},
        {name:'A generated rule enables decoherence',initial:[['Seed','A'],['B','Extra']],rule:[literal('Generate joint rule',[['Seed']],[[value(literal('Decohere',[['A'],['B']],[['C']]))]])],text:'A rule in a participating coherence can match a joint A/B binding. Read support does not add an extra consuming input position.'},
        {name:'A whole rule acquires an abstraction',initial:[[rule,'Use']],rule:[literal('Rule abstraction',[[rule]],[['Function']]),literal('Recognize concrete rule',[[rule]],[['Recognized']]),{name:'Use Function',input:[['Use','Function']],output:[{body:[literal('Return recognition',[['Recognized']],[['Done']])]}]}],text:'A whole rule derives Function. An inferred body receives the concrete rule witness and recognizes it by ordinary exact matching.'},
        {name:'A generated rule opens a nested body',initial:[['Seed','A','Extra']],rule:[literal('Generate body',[['Seed']],[[value({name:'Dynamic body',input:[['A']],output:[{body:[literal('Finish',[['Extra']],[['Done']])]}]})]])],text:'The invoked rule carries its lexical environment into a body. Read-only Seed is not added to the continuation’s consumed footprint.'},
        {name:'An escaped rule retains its local definition',initial:[['Enter','Make','Call']],rule:[{name:'Enter scope',input:[['Enter']],output:[{body:[literal('Local meaning',[['A']],[['Local']]),literal('Return closure',[['Make']],[[value({name:'Captured body',input:[['Call']],output:[{body:[literal('Return local',[['Local']],[['Done']])],particle:['A']} ]})]])]}]},literal('Global meaning',[['A']],[['Global']])],text:'A returned rule captures the local environment. Calling its body outside that scope can still use the local A → Local definition; the return continuation belongs to the caller.'},
        {name:'Consuming code later does not erase earlier results',initial:[[rule,'A','Erase']],rule:[literal('Remove code',[[rule,'Erase']],[[]])],text:'A rule can produce B, after which an explicit match can remove the rule value. The already completed event keeps its historical read justification.'}
    ];
    return {example};
})();
globalThis.kernel.example.push(...globalThis.kernel.dynamic.example);
