use std::collections::HashMap;
use failure::*;
use dynamic_typing::{ Scope, new_mutex_ref, ObjectType, CustomTypeObject, Type, FunctionType };
use ratel::{ ast as Ast };
use expressions::{ expression_to_string, determine_expression_type };
use statics::{ OBJECT_PROTOTYPE };
use error::{ AccessError };

pub fn type_from_properties(properties: &[Ast::Property], scope: &Scope) -> Result<Type, Error> {
    let properties: Result<HashMap<String, Type>, Error> = properties.iter().map(|property| {
        let transformed = match property {
            Ast::Property::Literal { key, value } => {
                (property_to_string(&key.item), determine_expression_type(&value, &scope)?)
            },

            Ast::Property::Shorthand(property) => {
                ((*property).to_string(), determine_expression_type(&Ast::Expression::Identifier(property), &scope)?)
            },

            Ast::Property::Method { key, value: _value } => {
                (property_to_string(&key.item), Type::from(FunctionType::new(vec!())))
            },

            Ast::Property::Spread { argument } => panic!("Property spread for Object literals is not implement!, {:#?}", argument),
        };

        Ok(transformed)
    }).collect();

    let mut properties = properties?;
    let prototype: Option<CustomTypeObject> = {
        let prototype = properties.get("__proto__");

        match prototype {
            Some(prototype) => {
                match prototype {
                    Type::Object(type_) => Some(CustomTypeObject::from(type_)),
                    Type::Function(type_) => Some(CustomTypeObject::from(type_)),
                    Type::Undefined => None,
                    _ => Some(CustomTypeObject::from(&*OBJECT_PROTOTYPE))
                }
            },
            None => Some(CustomTypeObject::from(&*OBJECT_PROTOTYPE)),
        }
    };

    properties.remove("__proto__");

    let new_type = Type::Object(new_mutex_ref(ObjectType::new(None, properties, prototype)));

    Ok(new_type)
}

pub fn determine_member_type(expression: &Ast::Expression, property: Ast::Node<'_, &str>, scope: &Scope) -> Result<Type, Error> {
    let object = determine_expression_type(expression, scope)?;
    let member_type = object.properties(|properties| {
        let mut member_type: Option<Type> = None;

        for (name, type_) in properties {
            if name != property.item {
                continue;
            }

            member_type = Some(type_.clone());
        }

        member_type
    });

    if let Some(type_) = member_type {
        Ok(type_)
    } else {
        Err(AccessError::UndefinedProperty { property: property.item.to_string(), object: object.to_string() }.into())
    }
}

fn property_to_string(property_key: &Ast::PropertyKey) -> String {
    match property_key {
        Ast::PropertyKey::Literal (value) => (*value).to_string(),
        Ast::PropertyKey::Binary (value) => (*value).to_string(),
        Ast::PropertyKey::Computed (expression_node) => expression_to_string(&expression_node.item),
    }
}
