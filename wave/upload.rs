use crate::failure::Failure;
use crate::mirror::Mirror;
use crate::setting::saturate;
use crate::table::Table;
use metal::device::{Device, Memory};

// A hashed index on the device, with its number of slots and how many things it indexes, so it is
// built again only once there are more.
pub struct Index {
    mirror: Mirror<u32>,
    pub slot: u32,
    count: usize,
}

impl Index {
    fn new(
        device: &Device,
        (slot, capacity): (Vec<u32>, usize),
        count: usize,
    ) -> Result<Self, Failure> {
        Ok(Self {
            mirror: Mirror::new(device, &slot)?,
            slot: saturate(capacity),
            count,
        })
    }

    pub fn memory(&self) -> &Memory {
        &self.mirror.memory
    }

    fn refresh(
        &mut self,
        device: &Device,
        count: usize,
        build: impl FnOnce() -> (Vec<u32>, usize),
    ) -> Result<(), Failure> {
        if self.count == count {
            return Ok(());
        }
        let (slot, capacity) = build();
        self.mirror.rewrite(device, &slot)?;
        self.slot = saturate(capacity);
        self.count = count;
        Ok(())
    }
}

// The tables as the kernels read them: the entries, the index of components' entries, the lone
// table and the roots it covers, the sizes, bases, coherences and reach of kinds, the inputs' rules
// and the rules' inputs, and the parts of several components with their members and catalog.
pub struct Upload {
    pub entry: Mirror<u32>,
    pub key: Index,
    pub lone: Mirror<u32>,
    pub size: Mirror<u32>,
    pub base: Mirror<u32>,
    pub coherence: Mirror<u32>,
    pub span: Mirror<u32>,
    pub reach: Mirror<u32>,
    pub rule: Mirror<u32>,
    pub arity: Mirror<u32>,
    pub part: Mirror<u32>,
    pub member: Mirror<u32>,
    pub catalog: Index,
    pub root: u32,
}

impl Upload {
    pub fn new(device: &Device, table: &Table) -> Result<Self, Failure> {
        Ok(Self {
            entry: Mirror::new(device, &table.entry)?,
            key: Index::new(device, table.key(), table.single.len())?,
            lone: Mirror::new(device, &table.lone)?,
            size: Mirror::new(device, &table.size)?,
            base: Mirror::new(device, &table.base)?,
            coherence: Mirror::new(device, &table.coherence)?,
            span: Mirror::new(device, &table.span)?,
            reach: Mirror::new(device, &table.reach)?,
            rule: Mirror::new(device, &table.rule)?,
            arity: Mirror::new(device, &table.arity)?,
            part: Mirror::new(device, &table.part)?,
            member: Mirror::new(device, &table.member)?,
            catalog: Index::new(device, table.catalog(), table.part.len())?,
            root: saturate(table.base.len()),
        })
    }

    // Brings the device's tables up to the host's once grounding changed them: the tables that only
    // grow are appended to, the lone table is written whole, and an index is built again when it
    // indexes more.
    pub fn refresh(&mut self, device: &Device, table: &mut Table) -> Result<(), Failure> {
        if !table.dirty {
            return Ok(());
        }
        self.entry.append(device, &table.entry)?;
        self.size.append(device, &table.size)?;
        self.base.append(device, &table.base)?;
        self.coherence.append(device, &table.coherence)?;
        self.span.append(device, &table.span)?;
        self.reach.append(device, &table.reach)?;
        self.part.append(device, &table.part)?;
        self.member.append(device, &table.member)?;
        self.lone.rewrite(device, &table.lone)?;
        self.key
            .refresh(device, table.single.len(), || table.key())?;
        self.catalog
            .refresh(device, table.part.len(), || table.catalog())?;
        self.root = saturate(table.base.len());
        table.dirty = false;
        Ok(())
    }
}
