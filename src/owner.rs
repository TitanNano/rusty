pub struct Owner<T> {
    proteges: Vec<T>,
}

impl<T> Owner<T> {
    pub fn own<'own>(&'own mut self, value: T) -> &'own T {
        self.proteges.push(value);

        self.proteges.last().unwrap()
    }

    pub fn own_mut(&mut self, value: T) -> &mut T {
        self.proteges.push(value);

        self.proteges.last_mut().unwrap()
    }

    pub fn new() -> Self {
        Owner { proteges: vec!() }
    }
}
