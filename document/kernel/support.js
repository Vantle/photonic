'use strict';
globalThis.kernel.support=(()=>{
    function consequence(rule){
        const remaining=rule.map(value=>value.premise.length),dependency=new Map(),answer=new Set(),pending=[];
        rule.forEach((value,index)=>{
            if(!value.premise.length)pending.push(value.head);
            for(const atom of value.premise)dependency.set(atom,[...(dependency.get(atom)||[]),index]);
        });
        for(let position=0;position<pending.length;position++){
            const atom=pending[position];if(answer.has(atom))continue;answer.add(atom);
            for(const index of dependency.get(atom)||[]){remaining[index]--;if(!remaining[index])pending.push(rule[index].head);}
        }
        return answer;
    }
    function evaluate(model){
        const answer=consequence(model.clause),atom=new Set(model.clause.flatMap(value=>[value.head,...value.premise]));
        return {status:atom=>answer.has(atom)?'supported':'unsupported',supported:[...answer].sort(),unsupported:[...atom].filter(value=>!answer.has(value)).sort(),closed:model.closed};
    }
    return {consequence,evaluate};
})();
