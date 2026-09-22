use crate::capture::{self, Capture};
use crate::construction::Construction;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::path::Path;
use crate::proof::Proof;
use crate::structure::Value;
use crate::support::{self, Address};
use crate::world;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct Qualification {
    proof: Arc<Proof>,
    construction: Construction<Capture, Capture>,
}

impl Qualification {
    pub fn new(path: Path, world: world::Identity) -> Result<Self, Failure> {
        let context = path
            .target()
            .world
            .get(&world)
            .ok_or(Failure::World(world))?
            .context;
        let context = capture::resolve(
            &path,
            &Address {
                derivation: vec![],
                state: path.record().len(),
            },
            context,
        )?;
        let history = path.target().history.clone();
        let proof = Arc::new(Proof { path, world });
        Ok(Self {
            proof: proof.clone(),
            construction: Construction {
                context: context.clone(),
                origin: Evidence {
                    read: BTreeMap::new(),
                    context: BTreeSet::new(),
                    history,
                    proof: Some(proof),
                    qualified: BTreeMap::new(),
                    capture: BTreeSet::from([context]),
                },
                capture: std::marker::PhantomData,
            },
        })
    }

    pub fn construction(&self) -> &Construction<Capture, Capture> {
        &self.construction
    }

    pub fn inspect(
        &self,
        request: support::Request,
    ) -> Result<Fragment<Value<Capture, Capture>>, Failure> {
        let support = support::resolve(&self.proof.path, self.proof.world, &request)?;
        let source = support.occurrence();
        let mut context = BTreeSet::new();
        source.value.context(&mut context);
        let context = context
            .into_iter()
            .map(|identity| {
                Ok((
                    identity,
                    capture::resolve(&self.proof.path, &request.address, identity)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Failure>>()?;
        let value = crate::activation::rename(source.value.clone(), &|identity| {
            Ok(context[&identity].clone())
        })?;
        let mut evidence = self.construction.evidence();
        evidence.capture.extend(context.into_values());
        evidence.qualified.insert(request.clone(), source.clone());
        Ok(Fragment { value, evidence })
    }
}
