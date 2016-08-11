mod datafield;

struct DataField<T> {
    pub data: Vec<T>
}

impl<T> DataField<T> {
    fn new<T>() -> DataField<T> {
        DataField<T> {
            data: Vec<T>::new()
        }
    }
}
