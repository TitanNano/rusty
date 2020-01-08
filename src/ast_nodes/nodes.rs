use rand::Rng;
use ratel::ast as Ast;
use super::super::dynamic_typing::{ Location };
use super::traits::*;
use super::super::expressions::{ expression_to_string };

#[derive(Clone, Debug)]
pub struct ExpressionNodeStruct<'ast> {
    id: u16,
    pub expression: Ast::Expression<'ast>,
    pub location: Location,
}

impl<'ast> ExpressionNodeStruct<'ast> {
    pub fn new_id() -> u16 {
        let mut rng = rand::thread_rng();

        rng.gen()
    }
}

impl<'ast> From<Ast::ExpressionNode<'ast>> for ExpressionNodeStruct<'ast> {
    fn from(value: Ast::ExpressionNode<'ast>) -> Self {
        let location = Location::from(*value);
        let expression = (*value).item;
        let id = Self::new_id();

        ExpressionNodeStruct { id, expression, location }
    }
}

impl<'ast> Node for ExpressionNodeStruct<'ast> {
    fn location(&self) -> &Location {
        &self.location
    }

    fn source(&self) -> String {
        expression_to_string(&self.expression)
    }
}

impl<'ast> ExpressionNode<'ast> for ExpressionNodeStruct<'ast> {
    fn expression(&self) -> &Ast::Expression<'ast> {
        &self.expression
    }
}

impl<'ast> NewExpressionNodeFromAst<'ast, ExpressionNodeStruct<'ast>> for ExpressionNodeStruct<'ast> {
    fn new(expression: Ast::Expression<'ast>, location: Location) -> Self  {
        let mut rng = rand::thread_rng();
        let id = rng.gen();

        Self { id, expression, location }
    }
}

pub struct StringNodeStruct {
    id: u16,
    value: String,
    location: Location,
}

impl Node for StringNodeStruct {
    fn source(&self) -> String {
        (&self.value).to_owned()
    }

    fn location(&self) -> &Location {
        &self.location
    }
}

impl<'ast> From<Ast::Node<'ast, &str>> for StringNodeStruct {
    fn from(value: Ast::Node<'ast, &str>) -> Self {
        let mut rng = rand::thread_rng();
        let location = Location::from(*value);
        let value = (*value).item.to_owned();
        let id = rng.gen();

        Self { id, value, location }
    }
}


impl<'ast> std::hash::Hash for ExpressionNodeStruct<'ast> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'ast> PartialEq for ExpressionNodeStruct<'ast> {
    fn eq(&self, value: &Self) -> bool {
        self.id == value.id
    }
}

impl<'ast> PartialEq for StringNodeStruct {
    fn eq(&self, value: &Self) -> bool {
        self.id == value.id
    }
}

impl<'ast> Eq for ExpressionNodeStruct<'ast> {}
impl<'ast> Eq for StringNodeStruct {}
