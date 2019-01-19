use std::collections::HashMap;
use uuid::Uuid;

use super::{ Variable, Type, ChangeTrace, TracedTypeMuation, Location, TracedChange };
use super::traits::CustomType;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct FunctionType {
    id: Uuid,
    name: Option<String>,
    arguments: Vec<Variable>,
    pub properties: HashMap<String, Type>,
    property_change_trace: ChangeTrace<TracedTypeMuation>,
    invocations: Vec<(Vec<Type>, Location)>,

}

impl FunctionType {
    pub fn new(arguments: Vec<Variable>) -> Self {
        FunctionType {
            id: Uuid::new_v4(),
            name: None,
            arguments,
            properties: HashMap::new(),
            property_change_trace: ChangeTrace::new(),
            invocations: vec!(),
        }
    }

    pub fn return_type(&self) -> Type {
        Type::Undefined
    }

    pub fn trace_invocation(&mut self, arguments: Vec<Type>, location: Location) {
        self.invocations.push((arguments, location));
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

impl TracedChange<TracedTypeMuation, Type, Location> for FunctionType {
    fn change(&mut self, change: TracedTypeMuation, new_type: Type, location: Location) {
        self.property_change_trace.change(change, new_type, location)
    }
}
