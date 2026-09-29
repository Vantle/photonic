use crate::failure::Failure;
use crate::setting::saturate;
use crate::table::Table;
use metal::device::{Device, Memory};

// The tables as uploaded for the kernels, with the slots of the hashed index and the roots the
// lone table covers.
pub struct Upload {
    pub entry: Memory,
    pub key: Memory,
    pub lone: Memory,
    pub size: Memory,
    pub base: Memory,
    pub mask: Memory,
    pub need: Memory,
    pub slot: u32,
    pub root: u32,
}

impl Upload {
    fn new(device: &Device, table: &Table) -> Result<Self, Failure> {
        let (key, slot) = table.key();
        Ok(Self {
            entry: device.upload(&table.entry)?,
            key: device.upload(&key)?,
            lone: device.upload(&table.lone)?,
            size: device.upload(&table.size)?,
            base: device.upload(&table.base)?,
            mask: device.upload(&table.mask)?,
            need: device.upload(&table.need)?,
            slot: saturate(slot),
            root: saturate(table.base.len()),
        })
    }
}

// The uploaded tables, uploaded again whenever grounding changed them.
pub fn refresh<'upload>(
    device: &Device,
    table: &mut Table,
    upload: &'upload mut Option<Upload>,
) -> Result<&'upload Upload, Failure> {
    let fresh = match upload.take() {
        Some(value) if !table.dirty => value,
        _ => Upload::new(device, table)?,
    };
    table.dirty = false;
    Ok(upload.insert(fresh))
}
