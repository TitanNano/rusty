extern crate ratel;
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate erased_serde;
extern crate uuid;
extern crate toolshed;
extern crate rand;
extern crate traitcast;
extern crate traitcast_core;

mod dynamic_typing;
mod statics;
mod error;
mod tracing;
mod expressions;
mod literals;
mod objects;
mod traveler;
mod validation;
mod ast_nodes;

use std::fs::File;
use std::io::prelude::*;
use std::iter::repeat;
use ratel::{ parse, ast as Ast };
use failure::*;

use dynamic_typing::{
    Type, Scope, Variable, VariableKind, CustomTypeObject,
};
use statics::{ OBJECT };
use tracing::{ tracing_pass };
use expressions::{ determine_expression_type };
use validation::{ validation_pass };
use error::ValidationError;
use std::sync::Arc;

fn main() {
    let global_object: Variable = Variable::new(String::from("Object"), (&*OBJECT).clone(), VariableKind::Const);

    let mut static_root_scope: Scope = Scope::new(String::from("StaticRoot"), None);

    static_root_scope.add(global_object);

    // read test.js
    let mut file = File::open("/Users/Jovan/rusty/test.js").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let structured_content: Vec<&str> = contents.split('\n').collect();

    // parse it
    let module = match parse(&contents) {
        Ok(ast) => ast,
        Err(e) => { println!("{:#?}", e); return; }
    };

    let module_body = module.body();

    let (module_scope, errors) = analyze_ast(module_body, &static_root_scope);

    // println!("{}", serde_json::to_string_pretty(&module_scope).unwrap());

    for error in errors {
        println!("Error while analyzing scope <{}>: {:?}", module_scope.name(), error);
    }

    let (mut module_scope, tracing_errors) = tracing_pass(module_body, module_scope);

    for error in tracing_errors {
        println!("Error while tracing scope <{}> for type changes: {:?}", module_scope.name(), error);
    }

    let mut validation_errors: Vec<Arc<ValidationError>> = validation_pass(module_body, &mut module_scope).into_iter().collect();

    validation_errors.sort_by(|a, b| {
        if a.location().start < b.location().start {
            return std::cmp::Ordering::Less
        }

        if a.location().start > b.location().start {
            return std::cmp::Ordering::Greater
        }

        std::cmp::Ordering::Equal
    });

    println!("{}", validation_errors.len());

    for error in validation_errors {
        let (line_start, column_start) = get_line_from_offset(error.location().start, &structured_content);
        let (_, column_end) = get_line_from_offset(error.location().end, &structured_content);
        let line_content = structured_content[line_start as usize];
        let mut range = column_end - column_start;

        if range < 0 {
            range = column_end;
        }

        let padding = repeat(" ").take(column_start as usize).collect::<String>();
        let locator = repeat("^").take(range as usize).collect::<String>();

        println!("Validation Error: {} at {}", error, get_line_from_offset_as_string(error.location().start, &structured_content));
        println!("{}\n{}{}", line_content, padding, locator);
    }

}

fn analyze_ast<'a, 'b>(body: Ast::StatementList, static_root_scope: &'a Scope<'b>) -> (Scope<'a>, Vec<Error>) {

    let mut module_scope = Scope::new(String::from("ModuleScope"), Some(static_root_scope));
    let mut scope_errors = vec!();

    for statement in body {
        let statement = **statement;

        if let Ast::Statement::Declaration(declaration_statement) = statement.item {
            let Ast::statement::DeclarationStatement { declarators: declarations, kind } = declaration_statement;

            for declaration in declarations {
                let variable = analyze_declaration(declaration.item, kind, &module_scope);

                match variable {
                    Ok(variable) => {

                        match variable.current_type() {
                            Type::Object(data) => module_scope.add_type(CustomTypeObject::from(data)),
                            Type::Function(data) => module_scope.add_type(CustomTypeObject::from(data)),
                            _ => ()
                        };

                        module_scope.add(variable)
                    },
                    Err(e) => scope_errors.push(e),
                }
            }
        }
    }

    (module_scope, scope_errors)
}

fn analyze_declaration(declaration: Ast::Declarator, kind: Ast::DeclarationKind, scope: &Scope) -> Result<Variable, Error> {
    let variable_name = match declaration.id.item {
        Ast::Pattern::Identifier(name) => name.to_string(),
        Ast::Pattern::RestElement { argument } => argument.item.to_string(),

        Ast::Pattern::ObjectPattern { ref properties, .. } => {
            let properties: Vec<Ast::Property> = properties.into_iter().map(|prop| prop.item).collect();

            return analyze_object_destructure(&properties[..], &declaration, kind);
        },

        Ast::Pattern::ArrayPattern { ref elements, .. } => {
            let elements: Vec<Ast::Pattern> = elements.into_iter().map(|element| { element.item }).collect();

            return analyze_array_destructure(&elements[..], &declaration, kind);
        },

        Ast::Pattern::AssignmentPattern { left: ref pattern, right: ref default } => {
            return analyze_assignment_pattern(pattern, default, &declaration, kind);
        },

        Ast::Pattern::Void => unreachable!("void pattern should only appear inside of array patterns!"),
    };

    let mut variable_type = match declaration.init {
        Some(value) => determine_expression_type(&value, scope)?,
        None => Type::Undefined,
    };

    variable_type.assign_name(&variable_name[..]);

    let variable = Variable::new(variable_name, variable_type, VariableKind::from(kind));

    Ok(variable)
}

fn analyze_object_destructure(_properties: &[Ast::Property], _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Object Destructuring is not implemented!");
}

fn analyze_array_destructure(_elements: &[Ast::Pattern], _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Array Destructuring is not implemented!");
}

fn analyze_assignment_pattern(_pattern: &Ast::Pattern, _default: &Ast::Expression, _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Assignment Patterns are not implemented!");
}

fn get_line_from_offset(location: u32, content: &[&str]) -> (i32, i32) {
    let mut counter = 0;
    let mut line_number = 0;

    for line in content {
        // add 1 here to account for the new line byte
        let line_lenght = (line.len() + 1) as u32;
        let future_counter = counter + line_lenght;

        if location >= future_counter {
            line_number += 1;
            counter = future_counter;

            continue;
        }

        let byte_column = (location - counter) as u32;
        // column starts at 1 not 0
        let column = (&line[..byte_column as usize]).chars().count() as i32;

        return (line_number, column);
    }

    (-1, -1)
}

fn get_line_from_offset_as_string(location: u32, content: &[&str]) -> String {
    let (line, column) = get_line_from_offset(location, content);

    line_column_as_string(line, column)
}

fn line_column_as_string(line: i32, column: i32) -> String {
    if line < 0 || column < 0 {
        return "out-of-bounds".to_string()
    }

    format!("{}:{}", line + 1, column + 1)
}
