use ast_nodes::ExpressionNodeStruct;
use dynamic_typing::{MutexRef, SafeBorrow, Type, Variable};
use error::ValidationError;
use std::clone::Clone;
use std::sync::Arc;

pub type MetaHashMap<'ast> =
    std::collections::HashMap<ExpressionNodeStruct<'ast>, MutexRef<MetaCarry<'ast>>>;

#[derive(Clone, Debug)]
pub struct MetaCarry<'ast> {
    error: Option<Arc<ValidationError>>,
    variable: Option<MutexRef<Variable>>,
    expression_type: Option<Type>,
    comparison: Option<ComparisonMeta<'ast>>,
    property_access: Option<PropertyAccessMeta<'ast>>,
}

#[derive(Clone, Debug)]
pub struct PropertyAccessMeta<'ast> {
    object: &'ast ExpressionNodeStruct<'ast>,
    property: &'ast ExpressionNodeStruct<'ast>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ComparisonType {
    Equality,
    Greater,
    Lesser,
}

#[derive(Clone, Debug)]
pub struct ComparisonMeta<'ast> {
    pub kind: ComparisonType,
    pub members: (ExpressionNodeStruct<'ast>, ExpressionNodeStruct<'ast>),
}

impl<'a> MetaCarry<'a> {
    pub fn new() -> Self {
        Self {
            error: None,
            variable: None,
            expression_type: None,
            comparison: None,
            property_access: None,
        }
    }

    pub fn expression_type(&self) -> Type {
        if let Some(ref variable) = self.variable {
            return variable.borrow_safe(|variable| variable.current_type().to_owned());
        }

        if let Some(ref expr_type) = self.expression_type {
            return expr_type.to_owned();
        }

        Type::Undefined
    }

    pub fn set_expression_type(&mut self, value: Type) {
        self.expression_type = Some(value);
    }

    pub fn error(&self) -> &Option<Arc<ValidationError>> {
        &self.error
    }

    pub fn adopt_errors(&mut self, other: &MutexRef<MetaCarry>) {
        other.borrow_safe(|other| {
            if other.error.is_none() {
                return;
            }

            self.set_error(other.error().as_ref().unwrap().clone());
        });
    }

    pub fn set_error(&mut self, value: Arc<ValidationError>) -> Option<Arc<ValidationError>> {
        if self.error.is_some() {
            return None;
        }

        self.error = Some(value.clone());

        Some(value)
    }

    pub fn variable(&self) -> &Option<MutexRef<Variable>> {
        &self.variable
    }

    pub fn set_variable(&mut self, value: Option<MutexRef<Variable>>) {
        self.variable = value;
    }

    pub fn comparison(&self) -> Option<ComparisonMeta<'a>> {
        self.comparison.clone()
    }

    pub fn set_comparison(&mut self, value: ComparisonMeta<'a>) {
        self.comparison = Some(value);
    }

    pub fn property_access(&self) -> &Option<PropertyAccessMeta<'a>> {
        &self.property_access
    }
}

impl<'ast> PropertyAccessMeta<'ast> {
    pub fn object(&self) -> &ExpressionNodeStruct<'ast> {
        self.object
    }

    pub fn property(&self) -> &ExpressionNodeStruct<'ast> {
        self.property
    }
}

impl<'ast> ComparisonMeta<'ast> {
    #[allow(unused)]
    pub fn kind(&self) -> &ComparisonType {
        &self.kind
    }

    pub fn members(&self) -> (&ExpressionNodeStruct<'ast>, &ExpressionNodeStruct<'ast>) {
        let (ref left, ref right) = self.members;

        (left, right)
    }
}
