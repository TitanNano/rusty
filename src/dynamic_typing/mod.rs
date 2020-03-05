mod types;
mod object_type;
mod function_type;
mod variable;
mod traits;
mod scope;
mod change_trace;
mod custom_type;

use std::sync::{ Arc, Mutex };
use std::ops::Deref;
use std::fmt::Debug;

pub use self::types::Type;
pub use self::object_type::ObjectType;
pub use self::variable::*;
pub use self::function_type::FunctionType;
pub use self::scope::Scope;
pub use self::scope::BindableScope;
pub use self::traits::*;
pub use self::change_trace::*;
pub use self::custom_type::*;

pub struct CompMutex<T> {
    inner: Mutex<T>,
}

pub type MutexRef<T> = Arc<CompMutex<Box<T>>>;

impl<T> CompMutex<T> {
    fn new(value: T) -> Self {
        Self { inner: Mutex::new(value) }
    }
}

impl<T> Deref for CompMutex<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl <T: Debug + 'static> Debug for CompMutex<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        (**self).fmt(formatter)
    }
}

impl<T: PartialEq> PartialEq for CompMutex<T> {
    fn eq(&self, other: &Self) -> bool {
        *self.try_lock().unwrap() == *other.try_lock().unwrap()
    }
}

impl <T: serde::Serialize> serde::Serialize for CompMutex<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error> where S: serde::Serializer {
        (**self).serialize(serializer)
    }
}

pub fn new_mutex_ref<T>(value: T) -> MutexRef<T> {
    Arc::new(CompMutex::new(Box::new(value)))
}
