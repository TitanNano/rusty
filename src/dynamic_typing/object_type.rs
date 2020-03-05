use std::collections::HashMap;
use super::{ Type, TracedTypeMuation, ChangeTrace, Location, TracedChange, CustomTypeObject };
use super::traits::{ CustomType };
use uuid::Uuid;

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ObjectType {
    id: Uuid,
    name: Option<String>,
    pub properties: HashMap<String, Type>,
    is_array: bool,
    prototype: Option<CustomTypeObject>,
    properties_change_trace: ChangeTrace<TracedTypeMuation>,
}

traitcast::traitcast!(struct ObjectType: CustomType);

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

    pub fn query_property(&self, property: &str, location: &Location) -> Option<Type> {
        let property_type = self.properties.get(property);

        let mutation = self.properties_change_trace.find(|change_set| {
            if location.end < change_set.loc.start {
                return false;
            }

            match &change_set.attribute {
                TracedTypeMuation::Add(name) => name == property,
                TracedTypeMuation::Remove(name) => name == property,
                TracedTypeMuation::Update(name) => name == property,
            }
        });

        if mutation.is_none() {
            if let Some(property_type) = property_type {
                return Some(property_type.to_owned())
            }

            return None;
        }

        let mutation = mutation.expect("we just checked, mutation has to be something!");

        match mutation.attribute {
            TracedTypeMuation::Remove(_) => None,
            TracedTypeMuation::Add(_) => Some(mutation.current_type.clone()),
            TracedTypeMuation::Update(_) => Some(mutation.current_type.clone()),
        }
    }
}

impl CustomType for ObjectType {
    fn assign_name(&mut self, name: String) {
        match self.name {
            Some(_) => {},
            None => self.name = Some(name),
        };
    }

    fn name(&self) -> &str {
        match &self.name {
            Some(name) => name,
            None => "ObjectLiteral",
        }
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn is_array(&self) -> bool {
        self.is_array
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
