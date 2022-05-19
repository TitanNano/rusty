use dynamic_typing::{ Scope, MutexRef, SafeBorrow  };
use expression_meta_data::MetaHashMap;
use error::ErrorVec;
use expression_meta_data::MetaCarry;
use ast_nodes::ExpressionNodeStruct;

pub struct Context<'own> {
    pub data_map: MutexRef<MetaHashMap<'own>>,
    pub scope: MutexRef<Scope>,
    pub errors: ErrorVec
}

impl<'own> Context<'own> {
    pub fn new(scope: MutexRef<Scope>, data_map: MutexRef<MetaHashMap<'own>>) -> Self {
        let errors = ErrorVec::new();

        Self { scope, data_map, errors }
    }


    pub fn clear_error(&mut self, meta: &MutexRef<MetaCarry>) {
        meta.borrow_safe(|meta| {
            if meta.error().is_none() {
                return;
            }

            self.errors.remove(meta.error().as_ref().unwrap());
        });
    }

    pub fn set_node_meta_data(&mut self, node: &ExpressionNodeStruct<'own>, meta: MutexRef<MetaCarry<'own>>) -> MutexRef<MetaCarry<'own>> {
        self.data_map.borrow_mut_safe(|map| map.insert(node.clone(), meta));
        self.data_map.borrow_safe(|map| map.get(node).unwrap().clone())
    }

    pub fn node_meta_data(&self, node: &ExpressionNodeStruct<'own>) -> MutexRef<MetaCarry<'own>> {
        self.data_map.borrow_safe(|map| map.get(node).unwrap().clone())
    }

    pub fn derive(&mut self, scope: &MutexRef<Scope>) -> Context<'own> {
        Context {
            data_map: self.data_map.clone(),
            scope: scope.clone(),
            errors: ErrorVec::new()
        }
    }

    #[allow(dead_code)]
    pub fn join(&mut self, other: Context) {
        self.errors.extend(other.errors);
    }
}
