use std::ops::Index;

#[derive(Debug)]
pub struct DataField<T> {
    pub data: Vec<T>
}

#[allow(dead_code)]
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

    pub fn last(&mut self) -> Option<&T> {
        self.data.last()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> Index<usize> for DataField<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.data[index]
    }
}
