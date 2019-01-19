use std::collections::HashMap;
use super::{ Type, TracedTypeMuation, ChangeTrace, Location, TracedChange, CustomTypeObject };
use super::traits::{ CustomType };
use uuid::Uuid;

#[derive(Debug, Serialize, PartialEq)]
pub struct ObjectType {
    id: Uuid,
    name: Option<String>,
    pub properties: HashMap<String, Type>,
    is_array: bool,
    prototype: Option<CustomTypeObject>,
    properties_change_trace: ChangeTrace<TracedTypeMuation>,
}

impl ObjectType {

    pub fn new(name: Option<String>, properties: HashMap<String, Type>, prototype: Option<CustomTypeObject>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            properties,
            prototype,
            is_array: false,
            properties_change_trace: ChangeTrace::new(),
        }
    }

    pub fn new_array(name: Option<String>, properties: HashMap<String, Type>, prototype: Option<CustomTypeObject>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            properties,
            prototype,
            is_array: true,
            properties_change_trace: ChangeTrace::new(),
        }
    }

    pub fn is_array(&self) -> bool {
        self.is_array
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
            Some(name) => name,
            None => "",
        }
    }

    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl TracedChange<TracedTypeMuation, Type, Location> for ObjectType {
    fn change(&mut self, change: TracedTypeMuation, new_type: Type, location: Location) {
        self.properties_change_trace.change(change, new_type, location)
    }
}

impl ToString for ObjectType {
    fn to_string(&self) -> String {
        self.name().to_string()
    }
}

// impl PartialEq for ObjectType {
//     fn eq(&self, other: &Self) -> bool {
//         let prototype = match self.prototype {
//             Some(self_prototype) => {
//                 if other.prototype.is_none() {
//                     return false;
//                 }
//
//                 let other_prototype = other.prototype.unwrap();
//
//                 *self_prototype.try_lock().unwrap() == *other_prototype.try_lock().unwrap()
//             },
//
//             None => other.prototype.is_none(),
//         };
//
//         self.id == other.id && self.name == other.name && self.properties == other.properties
//     }
// }
