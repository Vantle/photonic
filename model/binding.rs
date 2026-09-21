use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::scope;
use crate::slot::Slot;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding<Value> {
    scope: scope::Identity,
    entry: BTreeMap<usize, Fragment<Value>>,
    parent: Option<Arc<Self>>,
}

impl<Value: Clone> Binding<Value> {
    pub fn new(scope: scope::Identity) -> Self {
        Self {
            scope,
            entry: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn nested(&self, scope: scope::Identity) -> Result<Self, Failure> {
        let mut cursor = Some(self);
        while let Some(binding) = cursor {
            if binding.scope == scope {
                return Err(Failure::Scope(scope));
            }
            cursor = binding.parent.as_deref();
        }
        Ok(Self {
            scope,
            entry: BTreeMap::new(),
            parent: Some(Arc::new(self.clone())),
        })
    }

    pub fn bind(&self, slot: &Slot<Value>, value: Fragment<Value>) -> Result<Self, Failure> {
        if slot.scope != self.scope {
            return Err(Failure::Scope(slot.scope));
        }
        if self.entry.contains_key(&slot.position) {
            return Err(Failure::Occupied {
                scope: slot.scope,
                position: slot.position,
            });
        }
        let mut result = self.clone();
        result.entry.insert(slot.position, value);
        Ok(result)
    }

    pub fn resolve(&self, slot: &Slot<Value>) -> Result<Fragment<Value>, Failure> {
        let mut cursor = Some(self);
        while let Some(binding) = cursor {
            if binding.scope == slot.scope {
                return binding
                    .entry
                    .get(&slot.position)
                    .cloned()
                    .ok_or(Failure::Binding {
                        scope: slot.scope,
                        position: slot.position,
                    });
            }
            cursor = binding.parent.as_deref();
        }
        Err(Failure::Scope(slot.scope))
    }
}
