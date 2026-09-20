pub(crate) struct Store<Value> {
    value: Vec<Option<Value>>,
    vacant: Vec<usize>,
}

impl<Value> Store<Value> {
    pub fn new() -> Self {
        Self {
            value: Vec::new(),
            vacant: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: Value) -> usize {
        if let Some(index) = self.vacant.pop() {
            self.value[index] = Some(value);
            return index;
        }
        let index = self.value.len();
        self.value.push(Some(value));
        index
    }

    pub fn remove(&mut self, index: usize) -> Value {
        let value = self.value[index].take().unwrap();
        self.vacant.push(index);
        value
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.value.iter().flatten()
    }

    pub fn retained(&self) -> usize {
        self.value.len() + self.vacant.len()
    }
}

impl<Value> std::ops::Index<usize> for Store<Value> {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        self.value[index].as_ref().unwrap()
    }
}

impl<Value> std::ops::IndexMut<usize> for Store<Value> {
    fn index_mut(&mut self, index: usize) -> &mut Value {
        self.value[index].as_mut().unwrap()
    }
}
