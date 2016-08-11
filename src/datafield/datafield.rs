use std::ops::Index;

struct DataField<T> {
    pub data: Vec<T>
}

impl<T> DataField<T> {
    fn new() -> DataField<T> {
        DataField {
            data: Vec::<T>::new()
        }
    }
}

impl<T> Index<usize> for DataField<T> {
    type Output = T;

    fn index(&self, _index: usize) -> &T {
        &self.data[_index]
    }
}
