use std::collections::HashMap;
use super::{ Type };
use super::traits::{ CustomType };
use uuid::Uuid;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct ObjectType {
    id: Uuid,
    name: Option<String>,
    pub properties: HashMap<String, Type>,
    prototype: Option<Box<ObjectType>>,
}

impl ObjectType {

    pub fn new(name: Option<String>, properties: HashMap<String, Type>, prototype: Option<Box<ObjectType>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            properties,
            prototype
        }
    }
}

impl CustomType for ObjectType {
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

impl ToString for ObjectType {
    fn to_string(&self) -> String {
        self.name().to_string()
    }
}
