use std::fmt::Debug;
use erased_serde::Serialize;
use uuid::Uuid;

pub trait CustomType where Self: Serialize + Debug {
    fn assign_name(&mut self, name: String);
    fn name(&self) -> &str;
    fn id(&self) -> &Uuid;
}

serialize_trait_object!(CustomType);
