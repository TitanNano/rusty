use ratel::{ ast as Ast };

pub fn literal_to_string(literal: &Ast::Literal) -> String {
    match literal {
        Ast::Literal::String(literal) => (*literal).to_string(),
        Ast::Literal::Null => "null".to_string(),
        Ast::Literal::Number(literal) => (*literal).to_string(),
        Ast::Literal::Binary(literal) => (*literal).to_string(),
        Ast::Literal::RegEx(literal) => (*literal).to_string(),
        Ast::Literal::Undefined => "undefined".to_string(),
        Ast::Literal::True => "true".to_string(),
        Ast::Literal::False => "false".to_string(),
    }
}
