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
    properties_change_trace: ChangeTrace<TracedTypeMuation>,
    invocations: Vec<(Vec<Type>, Location)>,
}

traitcast::traitcast!(struct FunctionType: CustomType);

impl FunctionType {
    pub fn new(arguments: Vec<Variable>) -> Self {
        FunctionType {
            id: Uuid::new_v4(),
            name: None,
            arguments,
            properties: HashMap::new(),
            properties_change_trace: ChangeTrace::new(),
            invocations: vec!(),
        }
    }

    pub fn return_type(&self) -> Type {
        Type::Undefined
    }

    pub fn trace_invocation(&mut self, arguments: Vec<Type>, location: Location) {
        self.invocations.push((arguments, location));
    }

    pub fn query_property(&self, property: &str, location: &Location) -> Option<Type> {
        let mutation = self.properties_change_trace.find(|change_set| {
            if change_set.loc.start > location.end {
                return false;
            }

            match &change_set.attribute {
                TracedTypeMuation::Add(name) => name == property,
                TracedTypeMuation::Remove(name) => name == property,
                TracedTypeMuation::Update(name) => name == property,
            }
        });


        match mutation?.attribute {
            TracedTypeMuation::Remove(_) => None,
            TracedTypeMuation::Add(_) => Some(mutation?.current_type.clone()),
            TracedTypeMuation::Update(_) => Some(mutation?.current_type.clone()),
        }
    }
}

impl CustomType for FunctionType {
    fn assign_name(&mut self, name: String) {
        match self.name {
            Some(_) => {},
            None => self.name = Some(name),
        };
    }

    fn name(&self) -> &str {
        match &self.name {
            Some(name) => &name,
            None => "AnonymousFunction",
        }
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn is_array(&self) -> bool {
        false
    }
}

impl TracedChange<TracedTypeMuation, Type, Location> for FunctionType {
    fn change(&mut self, change: TracedTypeMuation, new_type: Type, location: Location) {
        self.properties_change_trace.change(change, new_type, location)
    }
}
