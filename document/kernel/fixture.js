'use strict';
window.kernel.fixture=()=>kernel.example.map((program,index)=>{
    const limit={state:index===7?12:80,world:4,cell:index===7?3:index===21?4:12,frame:10};
    const model=kernel.model.create(program,limit).run(12000),support=kernel.support.evaluate(model);
    return {
        name:program.name,program,limit,closed:model.closed,
        state:model.node.map(node=>({id:node.id,...node.state,status:support.status('s'+node.id)})),
        event:model.event.map(event=>({source:event.source,target:event.target,rule:event.name,status:support.status(event.id)}))
    };
});
document.getElementById('fixture')?.addEventListener('click',()=>{
    const url=URL.createObjectURL(new Blob([JSON.stringify(kernel.fixture())+'\n'],{type:'application/json'}));
    const link=document.createElement('a');link.href=url;link.download='reference.json';link.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
});
