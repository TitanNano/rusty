use uuid::Uuid;
use std::collections::HashMap;
use traitcast::{ cast_box };

use super::variable::Variable;
use error::ScopeError;
use super::traits::{ SafeBorrow };
use super::{ CustomTypeObject, new_mutex_ref, MutexRef, CustomTypeRef, CustomType, FunctionType, ObjectType };

#[derive(Serialize, Debug)]
pub struct Scope<'a> {
    name: String,
    type_declarations: HashMap<Uuid, CustomTypeObject>,
    variables: HashMap<String, MutexRef<Variable>>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    pub fn locate_own(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {

        if let Some(variable) = self.variables.get(variable_name) {
            return Ok(variable.to_owned());
        }

        Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
    }

    pub fn locate_chain(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        let result = self.locate_own(variable_name);

        if result.is_ok() {
            return result;
        }

        match self.parent {
            Some(parent) => {
                parent.locate_chain(variable_name)
            },
            None => Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
        }
    }

    pub fn locate(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        let result = self.locate_own(variable_name);

        if result.is_ok() {
            return result;
        }

        match self.parent {
            Some(parent) => {
                parent.locate_chain(variable_name)
            },
            None => Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
        }
    }

    pub fn add(&mut self, variable: Variable) {
        self.variables.insert(variable.name().to_owned(), new_mutex_ref(variable));
    }

    pub fn add_type(&mut self, type_def: CustomTypeObject) {
        let type_id = type_def.borrow_safe(|type_def| *type_def.id());

        if self.type_declarations.contains_key(&type_id) {
            return;
        }

        self.type_declarations.insert(type_id, type_def);
    }

    pub fn new(name: String, parent: Option<&'a Scope<'a>>) -> Self {
        Self { name, variables: HashMap::new(), parent, type_declarations: HashMap::new(), }
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

impl<'a> From<&'a Scope<'a>> for Scope<'a> {
    fn from(scope: &'a Scope<'a>) -> Self {
        Self::new("subscope".to_string(), Some(scope))
    }
}

pub trait BindableScope<T> {
    fn bind(&mut self, value: &T) -> T;
}

impl<'a> BindableScope<MutexRef<Variable>> for Scope<'a>{
    fn bind(&mut self, value: &MutexRef<Variable>) -> MutexRef<Variable> {
        let variable_name = value.borrow_safe(|value| { value.name().to_owned() });

        if let Some(variable) = self.variables.get(&variable_name) {
            return variable.to_owned();
        }

        self.add(value.borrow_safe(|inner_value| { (**inner_value).to_owned() }));

        self.variables.get(&variable_name).unwrap().to_owned()
    }
}

impl<'a> BindableScope<CustomTypeObject> for Scope<'a> {
    fn bind(&mut self, value: &CustomTypeObject) -> CustomTypeObject {
        if let Some(type_object) = self.type_declarations.get(&value.borrow_safe(|value| { value.id().to_owned() })) {
            return (*type_object).clone();
        }

        self.add_type(value.borrow_safe(|inner_value| {
            match inner_value {
                CustomTypeRef::Function(function_type) => CustomTypeObject::Function(new_mutex_ref(function_type.to_owned())),
                CustomTypeRef::Object(object_type) => CustomTypeObject::Object(new_mutex_ref(object_type.to_owned())),
            }
        }));

        self.type_declarations.get(&value.borrow_safe(|value| { value.id().to_owned() })).unwrap().to_owned()
    }
}


impl<'a, T: BindableScope<CustomTypeObject>> BindableScope<MutexRef<dyn CustomType>> for T {
    fn bind(&mut self, value: &MutexRef<dyn CustomType>) -> MutexRef<dyn CustomType> {
        let custom_type_object = CustomTypeObject::from(value);

        let custom_type_object = self.bind(&custom_type_object);

        let custom_type_ref = match custom_type_object {
            CustomTypeObject::Function(type_object) => cast_box::<MutexRef<FunctionType>, MutexRef<dyn CustomType>>(Box::new(type_object)),
            CustomTypeObject::Object(type_object) => cast_box::<MutexRef<ObjectType>, MutexRef<dyn CustomType>>(Box::new(type_object)),
        }.expect("we registered CustomType, FunctionType and Object type, so it should bee castable!");

        *custom_type_ref
    }
}
