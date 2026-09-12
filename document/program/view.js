'use strict';
(() => {
    const find=id=>document.getElementById(id);
    const choice=program.example.findIndex(value=>value.id===(new URLSearchParams(location.search).get('example')||'negate'));
    let index=choice<0?1:choice,graph=program.initial(program.example[index]),selected='v0',timer=null;
    function stop(){clearInterval(timer);timer=null;find('play').textContent='Play';find('play').setAttribute('aria-pressed','false');}
    function element(name,attribute={},text=''){
        const node=document.createElementNS('http://www.w3.org/2000/svg',name);
        for(const [key,value] of Object.entries(attribute))node.setAttribute(key,value);
        node.textContent=text;return node;
    }
    function history(version){
        const result=[];
        for(let value=version;value;value=graph.node.find(node=>node.id===value.parent))result.unshift(value);
        return result;
    }
    function draw(){
        const svg=find('graph'),depth=Math.max(...graph.node.map(value=>value.depth));
        const level=Array.from({length:depth+1},(_,index)=>graph.node.filter(value=>value.depth===index));
        const height=Math.max(240,Math.max(...level.map(value=>value.length))*110+60),width=Math.max(900,(depth+1)*230);
        svg.setAttribute('viewBox','0 0 '+width+' '+height);svg.style.width=width+'px';svg.style.height=height+'px';svg.replaceChildren();
        const position=new Map();
        for(const [column,value] of level.entries())for(const [row,node] of value.entries())position.set(node.id,{x:25+column*230,y:35+row*110+(height-60-value.length*110)/2});
        const active=history(graph.node.find(value=>value.id===selected)).map(value=>value.id);
        for(const event of graph.event){
            const from=position.get(event.source),into=position.get(event.target);
            const route=event.source===event.target?'M '+(from.x+120)+' '+from.y+' C '+(from.x+210)+' '+(from.y-35)+', '+(from.x-40)+' '+(from.y-35)+', '+(from.x+45)+' '+from.y:into.x<=from.x?'M '+(from.x+82)+' '+(from.y+72)+' C '+(from.x+82)+' '+(from.y+105)+', '+(into.x+82)+' '+(into.y+105)+', '+(into.x+82)+' '+(into.y+72):'M '+(from.x+165)+' '+(from.y+35)+' C '+(from.x+200)+' '+(from.y+35)+', '+(into.x-35)+' '+(into.y+35)+', '+into.x+' '+(into.y+35);
            const edge=element('path',{d:route,class:'edge'});
            if(active.includes(event.target)){edge.style.stroke='var(--accent)';edge.style.strokeWidth='2.5';}
            edge.append(element('title',{},event.nested?'Enter nested body':event.input.join('.')+' → '+event.output.join('.')));svg.append(edge);
        }
        const chosen=graph.event.find(value=>value.id===find('application').value);
        if(chosen&&chosen.evidence!==chosen.source){
            const from=position.get(chosen.evidence),into=position.get(chosen.target);
            const edge=element('path',{d:'M '+(from.x+82)+' '+(from.y+72)+' Q '+(into.x+185)+' '+(into.y+110)+', '+(into.x+82)+' '+(into.y+72),class:'edge','stroke-dasharray':'5 5'});
            edge.style.stroke='var(--good)';edge.append(element('title',{},'Evidence '+chosen.evidence+' licenses an application at '+chosen.source));svg.append(edge);
        }
        for(const value of graph.node){
            const point=position.get(value.id),text=program.display(value.particle);
            const group=element('g',{role:'button',tabindex:0,'aria-label':value.id+' '+text+' in '+value.scope,'aria-pressed':String(selected===value.id),class:selected===value.id?'selected':''});
            group.append(element('rect',{x:point.x,y:point.y,width:165,height:72,class:'particle'}),element('text',{x:point.x+82,y:point.y+20,class:'label'},value.id+' · '+value.scope),element('text',{x:point.x+82,y:point.y+47},text.length>22?text.slice(0,21)+'…':text),element('title',{},text));
            group.onclick=()=>{selected=value.id;render();};group.onkeydown=event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();selected=value.id;render();}};svg.append(group);
        }
    }
    function render(){
        const example=program.example[index],version=graph.node.find(value=>value.id===selected)||graph.node[0];
        find('question').textContent=example.question;
        find('program').textContent=example.rule.map(value=>value.input.join('.')+' → '+value.output.join('.')).join('\n')+'\n\n'+example.pattern.join('.')+' → '+(example.output?example.output.join('.'):'{\n'+example.body.map(value=>'    '+value.input.join('.')+' → '+value.output.join('.')).join('\n')+'\n}')+'\n\nInitial: '+example.input.join('.');
        find('version').replaceChildren();
        graph.node.forEach(value=>{const option=document.createElement('option');option.value=value.id;option.textContent=value.id+' · '+program.display(value.particle)+' · '+value.scope;find('version').append(option);});find('version').value=version.id;
        const pending=!graph.settled;
        find('progress').textContent='Round '+graph.round+' · '+graph.event.length+' events · '+graph.node.length+' states';
        find('next').disabled=!pending;
        find('status').textContent=graph.limited?'Paused at the state budget':pending?'Derivation continues':'No further enabled applications in this model';
        find('explanation').textContent=graph.limited?'Increase the budget and advance again. Pending work is retained; no future match has been ruled out.':'Selected '+version.id+' is shared by '+graph.event.filter(value=>value.target===version.id).length+' incoming applications. The displayed history is one representative discovery path; cycles and alternative histories remain as graph edges.';
        const previous=find('application').value;
        find('application').replaceChildren();
        const incoming=graph.event.filter(value=>value.target===version.id);
        for(const event of incoming){const option=document.createElement('option');option.value=event.id;option.textContent=event.source+' → '+event.target+' · '+event.rule;find('application').append(option);}
        if(incoming.some(value=>value.id===previous))find('application').value=previous;
        const application=incoming.find(value=>value.id===find('application').value);
        find('binding').textContent=program.display(version.particle);
        find('overlap').textContent=program.overlap(version.particle)?'Internal invariant failed: overlapping live resources.':'Only current resources are shown here. Intermediate deductions remain in the evidence graph.';
        find('output').textContent=(version.particle.map(value=>value.label+' uses origin '+value.origin.join(' + ')).join('\n')||'No remaining resources')+(version.control?.length?'\n\nConsumed by the enclosing application: '+version.control.join(' + '):'');
        find('proof').textContent=!application?'Initial source':application.path.length?'Application at '+application.source+' enabled by '+application.evidence+'\n'+application.justification.length+' evidence endpoint(s); incoming graph edges retain alternative derivations\n'+application.path.map(id=>{const event=graph.event.find(value=>value.id===id);return event.source+' → '+event.target+' : '+event.rule;}).join('\n'):'Direct match at '+application.source;
        if(application){
            const source=graph.node.find(value=>value.id===application.source);
            const show=selected=>selected.map(id=>{const token=source.particle.find(value=>value.id===id);return token.label+' @ '+id;}).join(', ')||'∅';
            find('proof').textContent+='\n\nConcrete footprint: '+show(application.footprint)+'\nConsumed here: '+show(application.selected)+(application.nested?'\nTransferred witness: '+show(application.transfer):'');
        }
        find('history').textContent=history(version).map((value,index)=>{
            const event=graph.event.find(item=>item.target===value.id);
            return (index?'  '+(event.nested?'enter body with source remainder':event.input.join('.')+' → '+event.output.join('.'))+' [discovered in round '+event.round+']\n':'')+value.id+'  '+program.display(value.particle)+'  ['+value.scope+']';
        }).join('\n');
        draw();
    }
    function reset(){stop();graph=program.initial(program.example[index]);selected='v0';render();}
    function next(){
        const count=graph.node.length;
        graph=program.advance(graph,program.example[index],{limit:Number(find('budget').value),visibility:find('visibility').value});
        if(graph.node.length>count)selected=graph.node.at(-1).id;
        render();if(graph.limited||graph.settled)stop();
    }
    program.example.forEach((value,index)=>{const option=document.createElement('option');option.value=index;option.textContent=value.name;find('example').append(option);});find('example').value=index;
    find('example').onchange=()=>{index=Number(find('example').value);reset();};
    find('visibility').onchange=reset;
    find('budget').onchange=()=>{stop();render();};
    find('application').onchange=render;
    find('version').onchange=()=>{selected=find('version').value;render();};
    find('reset').onclick=reset;find('next').onclick=()=>{stop();next();};
    find('play').onclick=()=>{if(timer!==null){stop();return;}find('play').textContent='Pause';find('play').setAttribute('aria-pressed','true');timer=setInterval(next,650);};
    render();const verification=program.verify();find('verification').textContent=verification.failure.length?'Model checks failed: '+verification.failure.join(', '):verification.checked+' model checks pass, including ownership-aware bindings, inferred source applications, canonical states, cycle closure, deep evidence, and pause/resume. This is a finite reference check, not a general soundness proof.';
})();
