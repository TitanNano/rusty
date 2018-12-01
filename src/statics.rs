use std::collections::HashMap;
use std::iter::FromIterator;

use dynamic_typing::{ Type, FunctionType, ObjectType, CustomType };

lazy_static! {
    pub static ref STRING_PROTOTYPE: Box<FunctionType> = {
        let mut func_type = Box::new(FunctionType::new(vec!()));

        (*func_type).assign_name(String::from("String"));

        func_type
    };
}

lazy_static! {
    pub static ref OBJECT_PROTOTYPE: Box<ObjectType> = {
        Box::new(ObjectType::new(Some(String::from("ObjectPrototype")), HashMap::from_iter(vec!(
                (String::from("name"), Type::String)
            )), None))
    };
}

lazy_static! {
    pub static ref ARRAY_PROTOTYPE: Box<ObjectType> = {
        Box::new(ObjectType::new(
            Some(String::from("ArrayPrototype")),
            HashMap::from_iter(vec!(
                (String::from("length"), Type::Number)
            )),
            Some((*OBJECT_PROTOTYPE).clone())
        ))
    };
}

lazy_static! {
    pub static ref OBJECT: Type = {
        let mut func_type = Box::new(FunctionType::new(vec!()));

        func_type.properties = HashMap::from_iter(vec!(
            (String::from("prototype"), (Type::from(&*OBJECT_PROTOTYPE)))
        ));

        func_type.assign_name(String::from("Object"));

        Type::Function(func_type)
    };
}
