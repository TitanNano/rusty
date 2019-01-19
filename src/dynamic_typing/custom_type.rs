use super::{ FunctionType, ObjectType, CustomType, MutexRef, SafeBorrow };
use uuid::Uuid;
use std::sync::Arc;

pub enum CustomTypeRef<'a> {
    Function(&'a FunctionType),
    Object(&'a ObjectType),
}

pub enum CustomTypeMutRef<'a> {
    Function(&'a mut FunctionType),
    Object(&'a mut ObjectType),
}

#[derive(Debug, Serialize, PartialEq)]
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

    pub fn borrow_mut_safe<B>(&mut self, closure: impl Fn(CustomTypeMutRef) -> B) -> B {
        match self {
            CustomTypeObject::Function(object) => object.borrow_mut_safe(|object_ref| {
                closure(CustomTypeMutRef::Function(object_ref))
            }),

            CustomTypeObject::Object(object) => object.borrow_mut_safe(|object_ref| {
                closure(CustomTypeMutRef::Object(object_ref))
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

    pub fn name(&self) -> &str {
        match self {
            CustomTypeRef::Function(object) => object.name(),
            CustomTypeRef::Object(object) => object.name(),
        }
    }
}

impl<'a> CustomTypeMutRef<'a> {
    pub fn id(&self) -> &Uuid {
        match self {
            CustomTypeMutRef::Function(object) => object.id(),
            CustomTypeMutRef::Object(object) => object.id(),
        }
    }

    pub fn assign_name(&mut self, name: String) {
        match self {
            CustomTypeMutRef::Function(object) => object.assign_name(name),
            CustomTypeMutRef::Object(object) => object.assign_name(name),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CustomTypeMutRef::Function(object) => object.name(),
            CustomTypeMutRef::Object(object) => object.name(),
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
