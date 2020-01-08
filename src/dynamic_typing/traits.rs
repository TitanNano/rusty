use std::fmt::Debug;
use std::cmp::PartialEq;
use std::sync::{ Mutex };

use erased_serde::Serialize;
use uuid::Uuid;
use super::change_trace::{ TracedTypeMuation, Location };
use super::types::Type;

pub trait CustomType where Self: Serialize + Debug + Sync + Send + TracedChange<TracedTypeMuation, Type, Location> {
    fn assign_name(&mut self, name: String);
    fn name(&self) -> &str;
    fn id(&self) -> &Uuid;
    fn is_array(&self) -> bool;
}

impl PartialEq for dyn CustomType {
    fn eq(&self, other: &dyn CustomType) -> bool {
        self.id() == other.id()
    }
}

pub trait TracedChange<TC, NV, L> {
    fn change(&mut self, change: TC, new_value: NV, location: L);
}

serialize_trait_object!(CustomType);

pub trait SafeBorrow<T> {
    fn borrow_safe<B, Func: FnOnce(&T) -> B>(&self, scope: Func) -> B;
    fn borrow_mut_safe<B, Func: FnOnce(&mut T) -> B>(&self, scope: Func) -> B;
}

impl<T> SafeBorrow<T> for Mutex<T> {
    fn borrow_safe<B, Func: FnOnce(&T) -> B>(&self, scope: Func) -> B {
        (scope)(&self.try_lock().unwrap())
    }

    fn borrow_mut_safe<B, Func: FnOnce(&mut T) -> B>(&self, scope: Func) -> B {
        scope(&mut self.try_lock().unwrap())
    }
}
