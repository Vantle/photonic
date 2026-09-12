'use strict';
window.kernel.example=(()=>{
    const literal=particle=>({particle});
    const rule=(name,input,output)=>({name,input,output:output.map(literal)});
    const boolean=[rule('True is Boolean',[['True']],[['Boolean']]),rule('False is Boolean',[['False']],[['Boolean']])];
    const body=(name,input,rule,particle=[])=>({name,input,output:[{body:rule,particle}]});
    const result=[
        {name:'Concrete And through two Boolean deductions',initial:[['And','True','False','Extra']],rule:[...boolean,body('And',[['And','Boolean','Boolean']],[rule('Both True',[['True','True']],[['True']]),rule('Mixed',[['True','False']],[['False']]),rule('Both False',[['False','False']],[['False']])])],text:'Derive the two-Boolean view, apply at the concrete And source, and transfer True.False into the body. The current configuration never accumulates a third operand.'},
        {name:'Two inputs broadcast their leftovers',initial:[['A','X'],['B','Y']],rule:[rule('Two outputs',[['A'],['B']],[['C'],['D']]),rule('Reunion',[['C'],['D']],[['E']])],text:'C and D both inherit X.Y. Their reunion carries X.Y once. Source inference may also apply the reunion directly at the original joint source.'},
        {name:'One source supports a joint sibling match',initial:[['A','X']],rule:[rule('Diverge',[['A']],[['B'],['C']]),rule('Decohere',[['B'],['C']],[['D']])],text:'A jointly produces B and C. The joint witness maps both input positions back to A once, enabling D.X at the concrete source.'},
        {name:'Competing histories cannot supply a joint match',initial:[['A']],rule:[rule('Left alternative',[['A']],[['B']]),rule('Right alternative',[['A']],[['C']]),rule('Requires both',[['B'],['C']],[['D']])],text:'B and C arise in different configurations. No joint witness exists, so D is not enabled.'},
        {name:'Explicit duplicate outputs remain distinct',initial:[['A']],rule:[rule('Produce two B coherences',[['A']],[['B'],['B']]),rule('Consume both',[['B'],['B']],[['D']])],text:'Equal output labels do not erase two explicit coherence positions. Their shared producer can justify an inferred D at A.'},
        {name:'Evidence-only co-results are not source leftovers',initial:[['A','Extra']],rule:[rule('Produce B and C',[['A']],[['B','C']]),rule('B to D',[['B']],[['D']])],text:'Applying B → D at concrete A.Extra yields D.Extra. Actually executing the producer and then B → D yields C.D.Extra. Both paths remain; they are not treated as equivalent.'},
        {name:'Independent branches compute independent equal results',initial:[['A','X']],rule:[rule('Diverge',[['A']],[['B'],['C']]),rule('Compute from X',[['X']],[['Y']]),rule('Reunion',[['B'],['C']],[['D']])],text:'Both coherences inherit X. If each computes X → Y independently, reunion retains two independently produced Ys. Carrying unchanged X through both gives one X.'},
        {name:'Fresh production and direct consumption',initial:[[]],rule:[rule('Create A',[[]],[['A']]),rule('A to B',[['A']],[['B']])],text:'Creation gives A a fresh introduction identity. A → B consumes A correctly. Repeated creation can grow genuinely new states and pause at a budget.'},
        {name:'Arbitrarily nested body expressions',initial:[['Not','True','Extra']],rule:[...boolean,body('Outer Not',[['Not','Boolean']],[body('Concrete True branch',[['True']],[body('Return from another body',[['Step']],[rule('Final literal',[['Finish']],[['False']])],['Finish'])],['Step'])])],text:'Three nested expression bodies return False.Extra. Continuations and lexical scope are explicit graph structure; there is no hard-coded one-body shape.'},
        {name:'A finite cycle shares its configurations',initial:[['A']],rule:[rule('Forward',[['A']],[['B']]),rule('Backward',[['B']],[['A']])],text:'A and B share a finite transition graph. Revisiting either state does not produce a fresh execution tree.'}
    ];
    return result;
})();
