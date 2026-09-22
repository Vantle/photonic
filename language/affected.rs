use crate::program::Symbol;

#[derive(Default)]
pub(crate) struct Set {
    frame: Vec<usize>,
    symbol: Vec<(usize, Symbol)>,
}

impl Set {
    pub fn clear(&mut self) {
        self.frame.clear();
        self.symbol.clear();
    }

    pub fn insert(&mut self, frame: usize, symbol: impl IntoIterator<Item = Symbol>) {
        self.frame.push(frame);
        self.symbol
            .extend(symbol.into_iter().map(|symbol| (frame, symbol)));
    }

    pub fn seal(&mut self) {
        self.frame.sort_unstable();
        self.frame.dedup();
        self.symbol.sort_unstable();
        self.symbol.dedup();
    }

    pub fn frame(&self) -> &[usize] {
        &self.frame
    }

    pub fn contains(&self, frame: usize) -> bool {
        self.frame.binary_search(&frame).is_ok()
    }

    pub fn includes(&self, frame: usize, symbol: Symbol) -> bool {
        self.symbol.binary_search(&(frame, symbol)).is_ok()
    }

    pub fn symbol(&self, frame: usize) -> impl Iterator<Item = Symbol> + '_ {
        let start = self.symbol.partition_point(|&(value, _)| value < frame);
        self.symbol[start..]
            .iter()
            .take_while(move |&&(value, _)| value == frame)
            .map(|&(_, symbol)| symbol)
    }

    pub fn retained(&self) -> usize {
        self.frame.len() + self.symbol.len()
    }
}

#[cfg(test)]
#[path = "test/affected.rs"]
mod test;
