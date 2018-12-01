use super::types::Type;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct Variable {
    name: String,
    current_type: Type,
}

impl Variable {
    pub fn current_type(&self) -> &Type {
        &self.current_type
    }

    pub fn change_type(&mut self, type_: Type) {
        self.current_type = type_;
    }

    pub fn name(&self)  -> &str {
        &self.name
    }

    pub fn new(name: String, current_type: Type) -> Self {
        Self {
            name,
            current_type,
        }
    }
}
