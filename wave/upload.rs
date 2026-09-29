use crate::failure::Failure;
use crate::mirror::Mirror;
use crate::setting::saturate;
use crate::table::Table;
use metal::device::Device;

// The tables as the kernels read them, with the slots of the hashed index, the parts it indexes and
// the roots the lone table covers.
pub struct Upload {
    pub entry: Mirror<u32>,
    pub key: Mirror<u32>,
    pub lone: Mirror<u32>,
    pub size: Mirror<u32>,
    pub base: Mirror<u32>,
    pub mask: Mirror<u64>,
    pub need: Mirror<u64>,
    pub slot: u32,
    pub root: u32,
    single: usize,
}

impl Upload {
    pub fn new(device: &Device, table: &Table) -> Result<Self, Failure> {
        let (key, slot) = table.key();
        Ok(Self {
            entry: Mirror::new(device, &table.entry)?,
            key: Mirror::new(device, &key)?,
            lone: Mirror::new(device, &table.lone)?,
            size: Mirror::new(device, &table.size)?,
            base: Mirror::new(device, &table.base)?,
            mask: Mirror::new(device, &table.mask)?,
            need: Mirror::new(device, &table.need)?,
            slot: saturate(slot),
            root: saturate(table.base.len()),
            single: table.single.len(),
        })
    }

    // Brings the device's tables up to the host's once grounding changed them: the tables that only
    // grow are appended to, the lone table is written whole, and the hashed index is built again
    // when it indexes more parts.
    pub fn refresh(&mut self, device: &Device, table: &mut Table) -> Result<(), Failure> {
        if !table.dirty {
            return Ok(());
        }
        self.entry.append(device, &table.entry)?;
        self.size.append(device, &table.size)?;
        self.base.append(device, &table.base)?;
        self.mask.append(device, &table.mask)?;
        self.lone.rewrite(device, &table.lone)?;
        if self.single != table.single.len() {
            let (key, slot) = table.key();
            self.key.rewrite(device, &key)?;
            self.slot = saturate(slot);
            self.single = table.single.len();
        }
        self.root = saturate(table.base.len());
        table.dirty = false;
        Ok(())
    }
}
