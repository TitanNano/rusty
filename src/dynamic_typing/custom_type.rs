use super::{ FunctionType, ObjectType, CustomType, MutexRef, SafeBorrow };
use uuid::Uuid;
use std::sync::Arc;

#[derive(Clone)]
pub enum CustomTypeRef<'a> {
    Function(&'a FunctionType),
    Object(&'a ObjectType),
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum CustomTypeObject {
    Function(MutexRef<FunctionType>),
    Object(MutexRef<ObjectType>),
}

impl CustomTypeObject {
    pub fn borrow_safe<B>(&self, closure: impl Fn(CustomTypeRef) -> B) -> B {
        match self {
            CustomTypeObject::Function(object) => object.borrow_safe(|object_ref| {
                closure(CustomTypeRef::Function(object_ref))
            }),

            CustomTypeObject::Object(object) => object.borrow_safe(|object_ref| {
                closure(CustomTypeRef::Object(object_ref))
            })
        }
    }
}

impl<'a> CustomTypeRef<'a> {
    pub fn id(&self) -> &Uuid {
        match self {
            CustomTypeRef::Function(object) => object.id(),
            CustomTypeRef::Object(object) => object.id(),
        }
    }
}

impl From<&MutexRef<ObjectType>> for CustomTypeObject {
    fn from(value: &MutexRef<ObjectType>) -> Self {
        CustomTypeObject::Object(Arc::clone(value))
    }
}

impl From<&MutexRef<FunctionType>> for CustomTypeObject {
    fn from(value: &MutexRef<FunctionType>) -> Self {
        CustomTypeObject::Function(Arc::clone(value))
    }
}

impl From<&MutexRef<dyn CustomType>> for CustomTypeObject {
    fn from(value: &MutexRef<dyn CustomType>) -> Self {
        if let Some(object_type) = traitcast::cast_ref::<MutexRef<dyn CustomType>, MutexRef<ObjectType>>(value) {
            return CustomTypeObject::Object(object_type.to_owned());
        }

        if let Some(function_type) = traitcast::cast_ref::<MutexRef<dyn CustomType>, MutexRef<FunctionType>>(value) {
            return CustomTypeObject::Function(function_type.to_owned());
        }

        panic!("unknown implementor of CustomType!");
    }
}
