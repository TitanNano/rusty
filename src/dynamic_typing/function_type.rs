use std::collections::HashMap;
use uuid::Uuid;

use super::{ Variable, Type };
use super::traits::CustomType;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct FunctionType {
    id: Uuid,
    name: Option<String>,
    arguments: Vec<Variable>,
    pub properties: HashMap<String, Type>
}

impl FunctionType {
    pub fn new(arguments: Vec<Variable>) -> Self {
        FunctionType {
            id: Uuid::new_v4(),
            name: None,
            arguments,
            properties: HashMap::new()
        }
    }
}

impl CustomType for FunctionType {
    fn assign_name(&mut self, name: String) {
        match self.name {
            Some(_) => return,
            None => self.name = Some(name),
        };
    }

    fn name(&self) -> &str {
        match &self.name {
            Some(name) => &name,
            None => "",
        }
    }

    fn id(&self) -> &Uuid {
        &self.id
    }
}
