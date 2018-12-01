use uuid::Uuid;
use std::collections::HashMap;

use super::variable::Variable;
use error::ScopeError;
use super::traits::CustomType;

#[derive(Serialize, Debug)]
pub struct Scope<'a> {
    name: String,
    type_declarations: HashMap<Uuid, Box<CustomType>>,
    pub variables: Vec<Variable>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn locate(&self, variable_name: &str) -> Result<&Variable, ScopeError> {
        for variable in &self.variables {
            if variable.name() != variable_name {
                continue;
            }

            return Ok(&variable);
        }

        match self.parent {
            Some(parent) => parent.locate(variable_name),
            None => Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
        }
    }

    pub fn add(&mut self, variable: Variable) {
        self.variables.push(variable);
    }

    pub fn add_type(&mut self, type_def: Box<CustomType>) {
        if self.type_declarations.contains_key(type_def.id()) {
            return;
        }

        self.type_declarations.insert(type_def.id().clone(), type_def);
    }

    pub fn new(name: String, parent: Option<&'a Scope<'a>>) -> Self {
        Self { name, variables: vec!(), parent, type_declarations: HashMap::new() }
    }
}

impl<'a> ToString for Scope<'a> {
    fn to_string(&self) -> String {
        let parent_name = match self.parent {
            Some(parent) => parent.to_string(),
            None => String::from("")
        };

        if parent_name.chars().count() == 0 {
            return self.name.to_string()
        }

        parent_name + " > " + &self.name
    }
}
