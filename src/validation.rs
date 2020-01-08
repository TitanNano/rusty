use ratel::{ ast as Ast };
use traveler::{ travel_ast, AstEvent };
use failure::*;
use dynamic_typing::{ Scope, Variable, Type, SafeBorrow, MutexRef, CustomType, Location };
use std::clone::Clone;
use std::sync::Arc;
use expressions::{ expression_to_string, determine_expression_type };
use error::{ ValidationError };
use ast_nodes::{ ExpressionNodeStruct, Node, ExpressionNode };

#[derive(Clone, Debug)]
struct MetaCarry {
    error: Option<Arc<Error>>,
    variable: Option<MutexRef<Variable>>,
    expression_type: Option<Type>,
}

impl MetaCarry {
    pub fn new() -> Self {
        Self { error: None, variable: None, expression_type: None, }
    }

    pub fn expression_type(&self) -> Type {
        if let Some(ref variable) = self.variable {
            return variable.borrow_safe(|variable| { variable.current_type().to_owned() });
        }

        if let Some(ref expr_type) = self.expression_type {
            return expr_type.to_owned();
        }

        Type::Undefined
    }

    pub fn set_expression_type(&mut self, value: Type) {
        self.expression_type = Some(value);
    }
}

type MetaHashMap<'ast> = std::collections::HashMap<ExpressionNodeStruct<'ast>, MetaCarry>;

pub fn validation_pass<'ast>(ast: Ast::StatementList<'ast>, scope: &Scope) -> Vec<ValidationError> {
    let mut errors: Vec<ValidationError> = vec!();
    let mut data_map: MetaHashMap<'ast> = std::collections::HashMap::new();
    let event_record = travel_ast(ast);

    for data in event_record.into_iter() {
        match data {
            AstEvent::Identifier { node, identifier } => {
                let variable = scope.locate(identifier);
                let mut meta_data = MetaCarry::new();

                let node_variable = match variable {
                    Ok(variable) => Some(variable.clone()),
                    Err(error) => { errors.push(ValidationError::from(error)); None },
                };

                meta_data.variable = node_variable;

                data_map.insert(node, meta_data);
            },

            AstEvent::Assignment { node, left, right } => {
                let mut meta_data = MetaCarry::new();
                let receiver_meta_data = data_map.get(&left).expect("there musst be receiver_meta_data!");
                let value_meta_data = data_map.get(&right).unwrap_or_else(|| panic!("every value should have value_meta_data {:#?}", right));

                let their_type = value_meta_data.expression_type();
                let own_type = receiver_meta_data.expression_type();

                meta_data.set_expression_type(their_type.clone());
                data_map.insert(node, meta_data);

                if own_type == their_type || own_type == Type::Null {
                    continue;
                }

                let validation_error = ValidationError::AssignTypeMissmatch {
                    target: left.source(),
                    own_type: own_type.to_string(),
                    their_type: their_type.to_string(),
                    location: left.location().clone(),
                };

                errors.push(validation_error);
            },

            AstEvent::PropertyAccess { object, property, node } => {
                let mut meta_data = MetaCarry::new();
                let object_type = determine_expression_type(&object.expression, scope).expect("variable has to exist at this location!");
                let property_result = object_type.query_property(&property.source(), property.location());

                if let Some(property_type) = property_result {

                    meta_data.set_expression_type(property_type);
                    data_map.insert(node, meta_data);

                    continue;
                }

                let node_location = node.location().clone();

                meta_data.set_expression_type(Type::Undefined);
                data_map.insert(node, meta_data);

                let validation_error = ValidationError::UnknownProperty {
                    object: expression_to_string(&object.expression),
                    property: property.source(),
                    location: node_location,
                };

                errors.push(validation_error);
            },

            AstEvent::Addition { node, left, right } =>  {
                let mut meta_data = MetaCarry::new();

                let (left_type, right_type) = {
                    let left_meta_data = data_map.get(&left).expect("we must have meta_data!");
                    let right_meta_data = data_map.get(&right).expect("we must have meta_data!");

                    (left_meta_data.expression_type(), right_meta_data.expression_type())
                };

                if left_type == Type::String || right_type == Type::String {
                    meta_data.set_expression_type(Type::String);
                    data_map.insert(node, meta_data);
                    continue;
                }

                if let Type::Composed { outer, .. } = left_type {
                    let is_array = outer.borrow_safe(|object| { object.is_array() });

                    if is_array {
                        meta_data.set_expression_type(Type::String);
                        data_map.insert(node, meta_data);
                        continue;
                    }
                }

                meta_data.set_expression_type(Type::Number);
                data_map.insert(node, meta_data);
            },

            AstEvent::Equality { node, left, right } => {
                let meta_data = MetaCarry::new();

                let (left_type, right_type) = {
                    let left_meta_data = data_map.get(&left).expect("we must have meta_data!");
                    let right_meta_data = data_map.get(&right).expect("we must have meta_data!");

                    (left_meta_data.expression_type(), right_meta_data.expression_type())
                };

                if left_type != right_type {
                    let validation_error = ValidationError::CompareTypeMissmatch {
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                        location: node.location().to_owned(),
                    };

                    errors.push(validation_error);
                }

                data_map.insert(node, meta_data);
            },

            AstEvent::Conditional { .. } => {},
            AstEvent::AlternateBody { .. } => {},
            AstEvent::AfterIf { .. } => {},
            AstEvent::ConsequentBody { .. } => {},
            AstEvent::Literal { node, literal } => {
                let mut meta_data = MetaCarry::new();
                let literal_type = Type::from(&literal);

                meta_data.set_expression_type(literal_type);
                data_map.insert(node, meta_data);
            },

            AstEvent::DynamicPropertyAccess { node, property, .. } => {
                let property_type = data_map.get(&property).expect("must have meta_data").expression_type();
                let mut meta_data = MetaCarry::new();

                if property_type != Type::String {
                    let error = ValidationError::InvalidType {
                        expression: property.source(),
                        current_type: property_type.to_string(),
                        expected_type: Type::String.to_string(),
                        location: node.location().to_owned(),
                    };

                    errors.push(error);
                }

                meta_data.set_expression_type(Type::Undefined);
                data_map.insert(node, meta_data);
            },

            AstEvent::Array { node, .. } => {
                let mut meta_data = MetaCarry::new();
                let expression_type = determine_expression_type(node.expression(), scope).expect("this is definetely an array!");

                meta_data.set_expression_type(expression_type);
                data_map.insert(node, meta_data);
            }

            AstEvent::This { .. } => {},
            AstEvent::Template { node, .. } => {
                let mut meta_data = MetaCarry::new();

                meta_data.set_expression_type(Type::String);
                data_map.insert(node, meta_data);
            },

            AstEvent::Sequence { node, sequence } => {
                let mut meta_data = MetaCarry::new();
                let last_item = sequence.into_iter().last().expect("sequences should have at least two items, or it wouldn't be a sequence");
                let expression_type = determine_expression_type(&last_item, scope).expect("there has to be a type!");

                meta_data.set_expression_type(expression_type);
                data_map.insert(node, meta_data);
            },

            AstEvent::FunctionCall { node, function, .. } => {
                let mut meta_data = MetaCarry::new();
                let function_definition = scope.locate(&expression_to_string(&function.expression)).expect("function should exist");

                let return_type = function_definition.borrow_safe(|definition| {
                    let def_type = definition.current_type();

                    match def_type {
                        Type::Function(data) => data.borrow_safe(|data| { data.return_type() }),
                        _ => panic!("only functions can be called!"),
                    }
                });

                meta_data.set_expression_type(return_type);
                data_map.insert(node, meta_data);
            },

            AstEvent::Spread { node, argument } => {
                let argument_meta_data = data_map.get(&argument).expect("there should be meta data!");
                let cloned_meta = (*argument_meta_data).clone();

                data_map.insert(node, cloned_meta);
            },

            AstEvent::PreOrPostFix { node, operand, .. } => {
                let meta_data = data_map.get(&operand).expect("there should be meta stuff").clone();

                data_map.insert(node, meta_data);
            },

            AstEvent::Object { node, .. } => {
                let mut meta_data = MetaCarry::new();
                let expression_type = determine_expression_type(node.expression(), &scope).expect("it should be possible to determine an object type");

                meta_data.set_expression_type(expression_type);
                data_map.insert(node, meta_data);
            },
            AstEvent::Function { .. } => {},
            AstEvent::Class { .. } => {},
        }
    };

    errors
}
