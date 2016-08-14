use std::ops::Index;

pub struct DataField<T> {
    pub data: Vec<T>
}

impl<T> DataField<T> {
    pub fn new() -> DataField<T> {
        DataField {
            data: Vec::new()
        }
    }

    pub fn push(&mut self, d: T) {
        self.data.push(d);
    }
}

impl<T> Index<usize> for DataField<T> {
    type Output = T;

    fn index(&self, _index: usize) -> &T {
        &self.data[_index]
    }
}
