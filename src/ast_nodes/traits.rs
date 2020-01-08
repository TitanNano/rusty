use super::super::dynamic_typing::{ Location };
use ratel::ast as Ast;

pub trait Node {
    fn source(&self) -> String;
    fn location(&self) -> &Location;
}

pub trait ExpressionNode<'ast>: Node {
    fn expression(&self) -> &Ast::Expression<'ast>;
}

pub trait NewExpressionNodeFromAst<'ast, T: ExpressionNode<'ast>> {
    fn new(expression: Ast::Expression<'ast>, loc: Location) -> T;
}


pub trait FromTrait<T> {
    fn from(value: T) -> Self;
}
