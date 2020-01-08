use std::collections::HashMap;

use super::{ ObjectType, FunctionType, SafeBorrow, MutexRef, new_mutex_ref, CustomTypeObject, Location };
use super::traits::CustomType;
use statics::{ OBJECT_PROTOTYPE };
use std::sync::{ Arc };
use ratel::{ ast as Ast };

//use serde::{ Serialize, Serializer };
//use serde::ser::{ SerializeStructVariant };

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum Type {
    Number,
    String,
    Boolean,
    RegExp,
    Object(MutexRef<ObjectType>),
    Function(MutexRef<FunctionType>),
    Undefined,
    Null,
    Mixed(Vec<Type>),
    Composed { outer: MutexRef<ObjectType>, inner: Box<Type> },
}

impl Type {
    pub fn unwrap(&self) -> Type {
        match self {
            Type::Composed { inner, .. } => {
                (&**inner).clone()
            },
            _ => self.clone(),
        }
    }

    pub fn properties<R, Func: Fn(&HashMap<String, Type>) -> R>(&self, closure: Func) -> R {
        match self {
            Type::Object(object) => object.borrow_safe(|object| {
                closure(&object.properties)
            }),
            Type::Function(object) => object.borrow_safe(|object| {
                closure(&object.properties)
            }),
            _ => OBJECT_PROTOTYPE.borrow_safe(|object| {
                closure(&object.properties)
            }),
        }
    }

    pub fn properties_mut<Func: Fn(&mut HashMap<String, Type>)>(&mut self, closure: Func) {
        match self {
            Type::Object(object) => object.borrow_mut_safe(|object| {
                closure(&mut object.properties);
            }),
            Type::Function(object) => object.borrow_mut_safe(|object| {
                closure(&mut object.properties);
            }),
            _ => panic!("unable to mutate properties of primitive type"),
        }
    }

    pub fn assign_name(&mut self, name: &str) {
        match self {
            Type::Object(data) => data.borrow_mut_safe(|data| data.assign_name(name.to_owned())),
            Type::Function(data) => data.borrow_mut_safe(|data| data.assign_name(name.to_owned())),
            _ => ()
        }
    }

    pub fn query_property(&self, property: &str, location: &Location) -> Option<Type> {
        match self {
            Type::Object(data) => data.borrow_safe(|data| data.query_property(property, location)),
            Type::Function(data) => data.borrow_safe(|data| data.query_property(property, location)),
            _ => None,
        }
    }
}

// impl Serialize for Type {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         match self {
//             Type::Number => serializer.serialize_unit_variant("Type", 0, "Number"),
//             Type::String => serializer.serialize_unit_variant("Type", 1, "String"),
//             Type::Boolean => serializer.serialize_unit_variant("Type", 2, "Boolean"),
//             Type::RegExp => serializer.serialize_unit_variant("Type", 3, "RegExp"),
//             Type::Object(object_data) => {
//                 let id = object_data.id();
//
//                 serializer.serialize_newtype_variant("Type", 4, "Object", id)
//             },
//             Type::Function(object_data) => {
//                 let id = object_data.id();
//
//                 serializer.serialize_newtype_variant("Type", 5, "Function", id)
//             },
//             Type::Undefined => serializer.serialize_unit_variant("Type", 6, "Undefined"),
//             Type::Mixed(value) => serializer.serialize_newtype_variant("Type", 7, "Mixed", value),
//             Type::Composed { outer, inner } => {
//                 let mut state = serializer.serialize_struct_variant("Type", 8, "Composed", 2)?;
//
//                 state.serialize_field("outer", outer.id())?;
//                 state.serialize_field("inner", inner)?;
//                 state.end()
//             }
//         }
//     }
// }


impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::String => "String".to_string(),
            Type::Number => "Number".to_string(),
            Type::Object(object) => object.borrow_safe(|object| object.name().to_owned()),
            Type::Function(object) => object.borrow_safe(|object| object.name().to_owned()),
            Type::RegExp => "RegExp".to_owned(),
            Type::Boolean => "Boolean".to_owned(),
            Type::Mixed(types) => types.iter().map(|type_| type_.to_string()).collect::<Vec<String>>().join(" | "),
            Type::Undefined => "Undefined".to_owned(),
            Type::Null => "Null".to_string(),
            Type::Composed { outer, inner } => format!("{}<{}>", outer.borrow_safe(|outer| outer.name().to_owned()), inner.to_string()),
        }
    }
}

impl From<&MutexRef<ObjectType>> for Type {
    fn from(mutex_type_ref: &MutexRef<ObjectType>) -> Self {
        Type::Object(Arc::clone(mutex_type_ref))
    }
}

impl From<MutexRef<ObjectType>> for Type {
    fn from(mutex_type: MutexRef<ObjectType>) -> Self {
        Type::Object(mutex_type)
    }
}

impl From<MutexRef<FunctionType>> for Type {
    fn from(mutex_type: MutexRef<FunctionType>) -> Self {
        Type::Function(mutex_type)
    }
}

impl From<&MutexRef<FunctionType>> for Type {
    fn from(mutex_type_ref: &MutexRef<FunctionType>) -> Self {
        Type::Function(Arc::clone(mutex_type_ref))
    }
}

impl From<CustomTypeObject> for Type {
    fn from(type_object: CustomTypeObject) -> Self {
        match type_object {
            CustomTypeObject::Function(mutex_type) => {
                Type::Function(mutex_type)
            },

            CustomTypeObject::Object(mutex_type) => {
                Type::Object(mutex_type)
            }
        }
    }
}

impl From<ObjectType> for Type {
    fn from(type_struct: ObjectType) -> Self {
        Type::Object(new_mutex_ref(type_struct))
    }
}

impl From<FunctionType> for Type {
    fn from(type_struct: FunctionType) -> Self {
        Type::Function(new_mutex_ref(type_struct))
    }
}

impl From<&Ast::Literal<'_>> for Type {
    fn from(literal: &Ast::Literal) -> Type {
        match literal {
            Ast::Literal::String(_) => Type::String,
            Ast::Literal::Number(_) => Type::Number,
            Ast::Literal::Null => Type::Null,
            Ast::Literal::True => Type::Boolean,
            Ast::Literal::RegEx(_) => Type::RegExp,
            Ast::Literal::False => Type::Boolean,
            Ast::Literal::Binary(_) => Type::Number,
            Ast::Literal::Undefined => Type::Undefined,
        }
    }
}
