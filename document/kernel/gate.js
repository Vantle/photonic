'use strict';
globalThis.kernel.gate=(()=>{
    function create(pattern){
        const candidate=pattern.map(()=>new Map());
        const prefix=Array.from({length:pattern.length+1},()=>new Map());
        prefix[0].set('[]',[]);
        const signature=value=>JSON.stringify(value.map(item=>[item.world,item.token.map(token=>token.id).sort()]));
        function compatible(binding,value,position){
            if(binding.some(item=>item.world===value.world))return false;
            const previous=binding.findLast(item=>JSON.stringify(pattern[item.position])===JSON.stringify(pattern[position]));
            return !previous||previous.world<value.world;
        }
        function* extend(binding,value,position){
            if(!compatible(binding,value,position))return;
            const result=[...binding,value],key=signature(result),next=position+1;
            if(prefix[next].has(key))return;
            prefix[next].set(key,result);
            if(next===pattern.length){yield result;return;}
            for(const item of candidate[next].values())yield* extend(result,item,next);
        }
        function* arrive(position,value){
            if(!Number.isInteger(position)||position<0||position>=pattern.length)throw new RangeError('Unknown pattern position');
            const item={...value,position,pattern:pattern[position]},key=signature([item]);
            if(candidate[position].has(key))return;
            candidate[position].set(key,item);
            for(const binding of prefix[position].values())yield* extend(binding,item,position);
        }
        return {arrive};
    }
    return {create};
})();
