use std::collections::HashMap;
use std::iter::FromIterator;

use dynamic_typing::{ Type, FunctionType, ObjectType, CustomType, MutexRef, new_mutex_ref,
    SafeBorrow, CustomTypeObject };

lazy_static! {
    pub static ref STRING_PROTOTYPE: MutexRef<FunctionType> = {
        let func_type = new_mutex_ref(FunctionType::new(vec!()));

        func_type.borrow_mut_safe(|func_type| func_type.assign_name("String".to_owned()));

        func_type
    };
}

lazy_static! {
    pub static ref OBJECT_PROTOTYPE: MutexRef<ObjectType> = {
        let properties = HashMap::from_iter(vec!(
            (String::from("name"), Type::String)
        ));

        let object_type = ObjectType::new(Some(String::from("ObjectPrototype")), properties, None);

        new_mutex_ref(object_type)
    };
}

lazy_static! {
    pub static ref ARRAY_PROTOTYPE: MutexRef<ObjectType> = {
        new_mutex_ref(ObjectType::new(
            Some(String::from("ArrayPrototype")),
            HashMap::from_iter(vec!(
                (String::from("length"), Type::Number)
            )),
            Some(CustomTypeObject::from(&*OBJECT_PROTOTYPE))
        ))
    };
}

lazy_static! {
    pub static ref OBJECT: Type = {
        let mut func_type = FunctionType::new(vec!());

        func_type.properties = HashMap::from_iter(vec!(
            (String::from("prototype"), (Type::from(&*OBJECT_PROTOTYPE)))
        ));

        func_type.assign_name("Object".to_owned());

        Type::Function(new_mutex_ref(func_type))
    };
}
