use std::collections::HashMap;

use super::{ ObjectType, FunctionType };
use super::traits::CustomType;
use statics::{ OBJECT_PROTOTYPE };
use serde::{ Serialize, Serializer };
use serde::ser::{ SerializeStructVariant };
use ecmascript::ast as Ast;

#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    Number,
    String,
    Boolean,
    RegExp,
    Object(Box<ObjectType>),
    Function(Box<FunctionType>),
    Undefined,
    Mixed(Vec<Type>),
    Composed { outer: Box<ObjectType>, inner: Box<Type> },
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

    pub fn properties(&self) -> &HashMap<String, Type> {
        match self {
            Type::Object(object) => &object.properties,
            Type::Function(object) => &object.properties,
            _ => &(*OBJECT_PROTOTYPE).properties,
        }
    }

    pub fn assign_name(&mut self, name: String) {
        match self {
            Type::Object(data) => data.assign_name(name),
            Type::Function(data) => data.assign_name(name),
            _ => ()
        }
    }
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            Type::Number => serializer.serialize_unit_variant("Type", 0, "Number"),
            Type::String => serializer.serialize_unit_variant("Type", 1, "String"),
            Type::Boolean => serializer.serialize_unit_variant("Type", 2, "Boolean"),
            Type::RegExp => serializer.serialize_unit_variant("Type", 3, "RegExp"),
            Type::Object(object_data) => {
                let id = object_data.id();

                serializer.serialize_newtype_variant("Type", 4, "Object", id)
            },
            Type::Function(object_data) => {
                let id = object_data.id();

                serializer.serialize_newtype_variant("Type", 5, "Function", id)
            },
            Type::Undefined => serializer.serialize_unit_variant("Type", 6, "Undefined"),
            Type::Mixed(value) => serializer.serialize_newtype_variant("Type", 7, "Mixed", value),
            Type::Composed { outer, inner } => {
                let mut state = serializer.serialize_struct_variant("Type", 8, "Composed", 2)?;

                state.serialize_field("outer", outer.id())?;
                state.serialize_field("inner", inner)?;
                state.end()
            }
        }
    }
}


impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::String => "string".to_string(),
            Type::Number => "number".to_string(),
            Type::Object(object) => object.name().to_string(),
            Type::Function(object) => object.name().to_string(),
            Type::RegExp => "RegExp".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::Mixed(types) => types.iter().map(|type_| type_.to_string()).collect::<Vec<String>>().join(" | "),
            Type::Undefined => "undefined".to_string(),
            Type::Composed { outer, inner } => format!("{}<{}>", outer.to_string(), inner.to_string()),
        }
    }
}

impl From<&Box<ObjectType>> for Type {
    fn from(boxed_type: &Box<ObjectType>) -> Self {
        Type::Object(boxed_type.clone())
    }
}

impl From<&Ast::Literal> for Type {
    fn from(literal: &Ast::Literal) -> Type {
        match literal {
            Ast::Literal::StringLiteral(_) => Type::String,
            Ast::Literal::NumericLiteral(_) => Type::Number,
            Ast::Literal::NullLiteral(_) => Type::Undefined,
            Ast::Literal::BooleanLiteral(_) => Type::Boolean,
            Ast::Literal::RegExpLiteral(_) => Type::RegExp,
        }
    }
}
