'use strict';
window.kernel.support=(()=>{
    function consequence(rule,assumption){
        const active=rule.filter(value=>!value.negative.some(atom=>assumption.has(atom))),remaining=active.map(value=>value.positive.length),dependency=new Map(),answer=new Set(),pending=[];
        active.forEach((value,index)=>{
            if(!value.positive.length)pending.push(value.head);
            for(const atom of value.positive)dependency.set(atom,[...(dependency.get(atom)||[]),index]);
        });
        for(let position=0;position<pending.length;position++){
            const atom=pending[position];if(answer.has(atom))continue;answer.add(atom);
            for(const index of dependency.get(atom)||[]){remaining[index]--;if(!remaining[index])pending.push(active[index].head);}
        }
        return answer;
    }
    const equal=(left,right)=>left.size===right.size&&[...left].every(value=>right.has(value));
    function evaluate(model){
        const rule=[...model.clause];
        if(!model.closed)for(const query of model.query.filter(value=>!value.closed)){
            rule.push({head:query.id,positive:['open/'+query.id],negative:[]},{head:'open/'+query.id,positive:[],negative:['open/'+query.id]});
        }
        const atom=new Set(rule.flatMap(value=>[value.head,...value.positive,...value.negative]));
        let lower=new Set(),upper=atom;
        for(;;){
            const next=consequence(rule,upper),bound=consequence(rule,next);
            if(equal(next,lower)&&equal(bound,upper))break;
            lower=next;upper=bound;
        }
        return {status:atom=>lower.has(atom)?'supported':upper.has(atom)?'conditional':'unsupported',supported:[...lower].sort(),conditional:[...upper].filter(value=>!lower.has(value)).sort(),unsupported:[...atom].filter(value=>!upper.has(value)).sort(),closed:model.closed};
    }
    function interpretation(model,limit=12){
        if(!model.closed)return {complete:false,reason:'Exploration is still open.',value:[]};
        if(model.query.length>limit)return {complete:false,reason:'Interpretation enumeration exceeds the display budget.',value:[]};
        const result=[];
        for(let mask=0;mask<2**model.query.length;mask++){
            const assumption=new Set(model.query.filter((_,index)=>Math.floor(mask/2**index)%2).map(value=>value.id));
            const answer=consequence(model.clause,assumption);
            if(model.query.every(value=>answer.has(value.id)===assumption.has(value.id)))result.push([...answer].sort());
        }
        return {complete:true,value:result};
    }
    return {consequence,evaluate,interpretation};
})();
