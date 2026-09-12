'use strict';
(()=>{
    const element=id=>document.getElementById(id);
    const example=element('example');
    kernel.example.forEach((value,index)=>example.add(new Option(value.name,String(index))));
    const requested=new URLSearchParams(location.search).get('example');
    if(requested!==null&&/^\d+$/.test(requested)&&Number(requested)<kernel.example.length)example.value=requested;
    let model,selected=0,previous=null;
    const button=(text,action)=>{const value=document.createElement('button');value.textContent=text;value.addEventListener('click',action);return value;};
    const describe=state=>kernel.state.show(state,label=>model.name.has(label)?'⟨'+model.name.get(label)+'⟩':label);
    function draw(event){
        const graph=element('graph');graph.replaceChildren();
        const make=(name,attribute,text)=>{
            const value=document.createElementNS('http://www.w3.org/2000/svg',name);
            for(const [key,item] of Object.entries(attribute))value.setAttribute(key,item);
            if(text!==undefined)value.textContent=text;
            graph.append(value);return value;
        };
        if(!event){make('text',{x:30,y:55},'Select an event to see its source, result, and supporting witness.');return;}
        make('line',{x1:260,y1:110,x2:730,y2:110,stroke:'#888','stroke-width':2});
        make('path',{d:'M 720 104 L 730 110 L 720 116',fill:'none',stroke:'#888'});
        for(const [position,id] of [[30,event.source],[730,event.target]]){
            make('rect',{x:position,y:70,width:235,height:80,fill:'white',stroke:'#888'});
            make('text',{x:position+15,y:100},'State '+id);
            const text=describe(model.node[id].state);
            const label=make('text',{x:position+15,y:127},text.length>29?text.slice(0,26)+'…':text);
            const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=text;label.append(title);
        }
        make('text',{x:330,y:90},event.name);
        make('text',{x:30,y:200},'Evidence: '+event.evidence.join(', '));
        make('text',{x:30,y:228},'The solid edge applies at its concrete source. Evidence is retained separately.');
    }
    function inspect(event){
        const evidence=event.evidence.map(id=>model.view.find(value=>value.id===id));
        element('detail').textContent=JSON.stringify({event:event.id,rule:event.name,source:event.source,target:event.target,binding:event.binding,evidence:evidence.map(value=>({id:value.id,source:value.source,witness:value.target,state:describe(model.node[value.target].state),flow:value.flow}))},null,2);
        draw(event);
    }
    function render(){
        const support=kernel.support.evaluate(model);
        element('status').textContent=`${model.node.length} configurations · ${model.event.length} events · ${model.view.length} witness mappings · ${model.work} agenda steps · ${model.closed?'finite fixed point':`${model.queued} queued, ${model.deferred} deferred`}`;
        element('state').replaceChildren();
        for(const node of model.node){
            const value=button(`${node.id} · ${describe(node.state)} · ${support.status('s'+node.id)}`,()=>{selected=node.id;render();});
            value.className='state';value.setAttribute('aria-pressed',String(node.id===selected));element('state').append(value);
        }
        element('selected').textContent=describe(model.node[selected].state)+'\n'+support.status('s'+selected);
        element('event').replaceChildren();
        for(const event of model.event.filter(value=>value.source===selected)){
            const row=document.createElement('div');
            row.append(button(`${event.id}: ${event.name} → state ${event.target}`,()=>inspect(event)),button('Follow',()=>{selected=event.target;render();inspect(event);}));
            element('event').append(row);
        }
        element('query').textContent=JSON.stringify(model.query.map(value=>({...value,status:support.status(value.id)})),null,2);
        element('advance').disabled=model.closed;element('explore').disabled=model.closed;
        element('expand').disabled=model.closed;
        element('revision').disabled=!kernel.example[Number(example.value)].later||previous!==null;
        element('comparison').textContent=previous?`Previous snapshot: ${previous.node.length} configurations, ${previous.event.length} events. Current snapshot adds a rule. The previous graph is retained unchanged in memory; Reset restores its program.`:'';
        element('detail').textContent='Select an outgoing event to inspect its witness and footprint.';draw();
    }
    function reset(){
        const value=kernel.example[Number(example.value)];
        model=kernel.model.create(value,{state:40,world:4,cell:7,frame:10});selected=0;previous=null;
        element('description').textContent=value.text;element('program').textContent=JSON.stringify(value,null,2);render();
    }
    example.addEventListener('change',reset);element('reset').addEventListener('click',reset);
    let gate,alternative,count,received;
    function pulse(){element('pulse').textContent=`${received} arrivals · ${count} completed bindings. Repeated evidence does not fire again.`;}
    function clear(){gate=kernel.gate.create([['A'],['B']]);alternative=kernel.gate.create([['A'],['B']]);count=0;received=0;pulse();}
    function arrive(target,position){
        received++;
        count+=[...target.arrive(position,{world:position,token:[{id:'operand'+position,label:position?'B':'A'}]})].length;
        pulse();
    }
    element('arrival').addEventListener('click',()=>arrive(gate,0));
    element('second').addEventListener('click',()=>arrive(gate,1));
    element('alternative').addEventListener('click',()=>arrive(alternative,1));
    element('clear').addEventListener('click',clear);
    element('advance').addEventListener('click',()=>{model.run(100);render();});
    element('explore').addEventListener('click',()=>{model.run(2000);render();});
    element('expand').addEventListener('click',()=>{const limit=model.limit;model.run(2000,{state:limit.state+20,cell:limit.cell+2,frame:limit.frame+2});render();});
    element('revision').addEventListener('click',()=>{
        const value=kernel.example[Number(example.value)];previous=model;
        const revised={...value,rule:[...value.rule,...value.later]};
        model=kernel.model.create(revised).run(2000);selected=0;element('program').textContent=JSON.stringify(revised,null,2);render();
    });
    reset();clear();
})();
