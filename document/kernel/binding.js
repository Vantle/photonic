'use strict';
globalThis.kernel.binding=(()=>{
    function* particle(pattern,value,selected=[],state){
        if(!pattern.length){yield selected;return;}
        const expected=pattern[0],label=typeof expected==='string'?expected:expected.label;
        const matches=token=>token.label===label&&(typeof expected==='string'||(expected.environment===undefined?token.capture===expected.capture:token.capture!==undefined&&kernel.value.environment(state,token.capture)===expected.environment));
        const previous=selected.filter(matches).at(-1);
        for(const token of value){
            if(!matches(token)||selected.some(value=>value.id===token.id)||previous&&previous.id>=token.id)continue;
            yield* particle(pattern.slice(1),value,[...selected,token],state);
        }
    }
    function* world(pattern,value,frame){
        if(!pattern.length){yield [];return;}
        const gate=kernel.gate.create(pattern);
        for(let index=0;index<value.world.length;index++){
            const current=value.world[index];
            if(current.frame!==frame)continue;
            for(let position=0;position<pattern.length;position++){
                for(const token of particle(pattern[position],current.particle,[],value))yield* gate.arrive(position,{world:index,token});
            }
        }
    }
    return {particle,world};
})();
