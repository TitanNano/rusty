use uuid::Uuid;
use std::collections::HashMap;
use traitcast::cast_box;

use super::variable::Variable;
use error::ScopeError;
use super::traits::SafeBorrow;
use super::{ CustomTypeObject, new_mutex_ref, MutexRef, CustomTypeRef, CustomType, FunctionType, ObjectType };

#[derive(Serialize, Debug)]
pub struct Scope {
    name: String,
    type_declarations: HashMap<Uuid, CustomTypeObject>,
    variables: HashMap<String, MutexRef<Variable>>,
    parent: Option<MutexRef<Scope>>,
}

impl Scope {
    pub fn new(name: String, parent: Option<MutexRef<Scope>>) -> Self {
        Self { name, variables: HashMap::new(), parent, type_declarations: HashMap::new(), }
    }
}

pub type ScopeRef = MutexRef<Scope>;

pub trait Scoped {
    fn name(&self) -> String;
    fn set_name(&mut self, value: String);
    fn locate_own(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError>;
    fn locate_chain(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError>;
    fn locate(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError>;
    fn add(&mut self, variable: Variable);
    fn add_type(&mut self, type_def: CustomTypeObject);
}

impl Scoped for Scope {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn set_name(&mut self, value: String) {
        self.name = value;
    }

    fn locate_own(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {

        if let Some(variable) = self.variables.get(variable_name) {
            return Ok(variable.to_owned());
        }

        Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
    }

    fn locate_chain(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        let result = self.locate_own(variable_name);

        if result.is_ok() {
            return result;
        }

        match &self.parent {
            Some(parent) => {
                parent.borrow_safe(|scope| scope.locate_chain(variable_name))
            },
            None => Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
        }
    }

    fn locate(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        let result = self.locate_own(variable_name);

        if result.is_ok() {
            return result;
        }

        match &self.parent {
            Some(parent) => {
                parent.borrow_safe(|scope| scope.locate_chain(variable_name))
            },
            None => Err(ScopeError::UndefinedVariable { variable_name: variable_name.to_string(), scope_name: self.to_string() })
        }
    }

    fn add(&mut self, variable: Variable) {
        self.variables.insert(variable.name().to_owned(), new_mutex_ref(variable));
    }

    fn add_type(&mut self, type_def: CustomTypeObject) {
        let type_id = type_def.borrow_safe(|type_def| *type_def.id());

        if self.type_declarations.contains_key(&type_id) {
            return;
        }

        self.type_declarations.insert(type_id, type_def);
    }
}

impl Scoped for ScopeRef {
    fn name(&self) -> String {
        self.borrow_safe(|scope| scope.name())
    }

    fn set_name(&mut self, value: String) {
        self.borrow_mut_safe(|scope| scope.set_name(value))
    }

    fn locate_own(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        self.borrow_safe(|scope| scope.locate_own(variable_name))
    }

    fn locate_chain(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        self.borrow_safe(|scope| scope.locate_chain(variable_name))
    }

    fn locate(&self, variable_name: &str) -> Result<MutexRef<Variable>, ScopeError> {
        self.borrow_safe(|scope| scope.locate(variable_name))
    }

    fn add(&mut self, variable: Variable) {
        self.borrow_mut_safe(|scope| scope.add(variable))
    }

    fn add_type(&mut self, type_def: CustomTypeObject) {
        self.borrow_mut_safe(|scope| scope.add_type(type_def))
    }
}

impl ToString for Scope {
    fn to_string(&self) -> String {
        let parent_name = match &self.parent {
            Some(parent) => parent.borrow_safe(|scope| scope.to_string()),
            None => String::from("")
        };

        if parent_name.chars().count() == 0 {
            return self.name.to_string()
        }

        parent_name + " > " + &self.name
    }
}

impl From<MutexRef<Scope>> for Scope {
    fn from(scope: MutexRef<Scope>) -> Self {
        Self::new("subscope".to_string(), Some(scope))
    }
}

pub trait BindableScope<T> {
    fn bind(&mut self, value: &T) -> T;
}

impl BindableScope<MutexRef<Variable>> for Scope {
    fn bind(&mut self, value: &MutexRef<Variable>) -> MutexRef<Variable> {
        let variable_name = value.borrow_safe(|value| { value.name().to_owned() });

        if let Some(variable) = self.variables.get(&variable_name) {
            return variable.to_owned();
        }

        self.add(value.borrow_safe(|inner_value| { (**inner_value).to_owned() }));

        self.variables.get(&variable_name).unwrap().to_owned()
    }
}

impl BindableScope<MutexRef<Variable>> for MutexRef<Scope> {
    fn bind(&mut self, value: &MutexRef<Variable>) -> MutexRef<Variable> {
        self.borrow_mut_safe(|scope| scope.bind(value))
    }
}

impl BindableScope<CustomTypeObject> for Scope {
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

// this currently connflicts with the impl below
// impl<V, S> BindableScope<V> for MutexRef<S> where S: BindableScope<V> {
//     fn bind(&mut self, value: &V) -> V {
//         self.borrow_mut_safe(|scope| scope.bind(value))
//     }
// }

impl<T: BindableScope<CustomTypeObject>> BindableScope<MutexRef<dyn CustomType>> for T {
    fn bind(&mut self, value: &MutexRef<dyn CustomType>) -> MutexRef<dyn CustomType> {
        let custom_type_object = CustomTypeObject::from(value);

        let custom_type_object = self.bind(&custom_type_object);

        let custom_type_ref = match custom_type_object {
            CustomTypeObject::Function(type_object) => cast_box::<MutexRef<FunctionType>, MutexRef<dyn CustomType>>(Box::new(type_object)),
            CustomTypeObject::Object(type_object) => cast_box::<MutexRef<ObjectType>, MutexRef<dyn CustomType>>(Box::new(type_object)),
        }.expect("we registered CustomType, FunctionType and Object type, so it should be castable!");

        *custom_type_ref
    }
}
