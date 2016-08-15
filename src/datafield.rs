use std::ops::Index;

#[derive(Debug)]
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

    pub fn first(&mut self) -> Option<&T> {
        self.data.first()
    }
}

impl<T> Index<usize> for DataField<T> {
    type Output = T;

    fn index(&self, _index: usize) -> &T {
        &self.data[_index]
    }
}
