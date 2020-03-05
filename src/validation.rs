use ratel::{ ast as Ast };
use traveler::{ travel_ast, AstEvent, travel_ast_statement };
use dynamic_typing::{
    Scope, Variable, Type, SafeBorrow, MutexRef, CustomType, TracedChange, FunctionType,
    TracedTypeMuation, TracedTypeChange, Location, BindableScope, ObjectType  };
use std::clone::Clone;
use std::sync::Arc;
use expressions::{ expression_to_string, determine_expression_type };
use error::{ ValidationError, ErrorVec };
use ast_nodes::{ ExpressionNodeStruct, Node, ExpressionNode };
use traitcast::cast_ref;
use std::collections::hash_set::HashSet;

#[derive(Clone, Debug)]
struct MetaCarry<'ast> {
    error: Option<Arc<ValidationError>>,
    variable: Option<MutexRef<Variable>>,
    expression_type: Option<Type>,
    equality: Option<Type>,
    subject: Option<ExpressionNodeStruct<'ast>>,
    property_access: Option<PropertyAccessMeta<'ast>>,
}

#[derive(Clone, Debug)]
struct PropertyAccessMeta<'ast> {
    object: &'ast ExpressionNodeStruct<'ast>,
    property: &'ast ExpressionNodeStruct<'ast>,
}

impl<'a> MetaCarry<'a> {
    pub fn new() -> Self {
        Self {
            error: None,
            variable: None,
            expression_type: None,
            equality: None,
            subject: None,
            property_access: None,
        }
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

    pub fn error(&self) -> &Option<Arc<ValidationError>> {
        &self.error
    }

    pub fn adopt_errors(&mut self, other: &MetaCarry) {
        if other.error.is_none() {
            return;
        }

        self.set_error(other.error().as_ref().unwrap().clone());
    }

    pub fn set_error(&mut self, value: Arc<ValidationError>) -> Option<Arc<ValidationError>> {
        if self.error.is_some() {
            return None
        }

        self.error = Some(value.clone());

        Some(value)
    }
}

type MetaHashMap<'ast> = std::collections::HashMap<ExpressionNodeStruct<'ast>, MetaCarry<'ast>>;

pub fn validation_pass<'ast>(ast: Ast::StatementList<'ast>, scope: &mut Scope) -> ErrorVec {
    let mut errors: ErrorVec = HashSet::new();
    let mut data_map: MetaHashMap<'ast> = std::collections::HashMap::new();
    let event_record = travel_ast(ast);

    validate_events(event_record, &mut data_map, scope, &mut errors);

    errors
}


fn validate_events<'ast>(event_record: Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>>, data_map: &mut MetaHashMap<'ast>, scope: &mut Scope, errors: &mut ErrorVec) {
    for data in event_record.into_iter() {
        match data {
            AstEvent::Identifier { node, identifier } => {
                let variable = scope.locate(identifier);
                let mut meta_data = MetaCarry::new();

                let node_variable = match variable {
                    Ok(variable) => Some(variable.clone()),
                    Err(error) => {
                        let error = ValidationError::from(error);
                        let error = meta_data.set_error(error.into());

                        if let Some(error) = error {
                            errors.insert(error);
                        }

                        None
                    },
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
                meta_data.adopt_errors(value_meta_data);
                meta_data.adopt_errors(receiver_meta_data);

                if own_type == their_type || own_type == Type::Null {
                    data_map.insert(node, meta_data);
                    continue;
                }

                let validation_error = ValidationError::AssignTypeMissmatch {
                    target: left.source(),
                    own_type: own_type.to_string(),
                    their_type: their_type.to_string(),
                    location: left.location().clone(),
                };

                let validation_error = meta_data.set_error(validation_error.into());

                if let Some(validation_error) = validation_error {
                    errors.insert(validation_error);
                }

                data_map.insert(node, meta_data);
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

                let node_location = property.location().clone();

                meta_data.set_expression_type(Type::Undefined);

                let validation_error = ValidationError::UnknownProperty {
                    object: expression_to_string(&object.expression),
                    property: property.source(),
                    location: node_location,
                };

                let validation_error = meta_data.set_error(validation_error.into());

                if let Some(validation_error) = validation_error {
                    errors.insert(validation_error);
                }

                data_map.insert(node, meta_data);
            },

            AstEvent::Addition { node, left, right } =>  {
                let mut meta_data = MetaCarry::new();
                let left_meta_data = data_map.get(&left).expect("we must have meta_data!");
                let right_meta_data = data_map.get(&right).expect("we must have meta_data!");

                let (left_type, right_type) = (left_meta_data.expression_type(), right_meta_data.expression_type());

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
                meta_data.adopt_errors(left_meta_data);
                meta_data.adopt_errors(right_meta_data);

                data_map.insert(node, meta_data);
            },

            AstEvent::Equality { node, left, right } => {
                let mut meta_data = MetaCarry::new();

                let left_meta_data = data_map.get(&left).expect("we must have meta_data!");
                let right_meta_data = data_map.get(&right).expect("we must have meta_data!");

                let (left_type, right_type) = (left_meta_data.expression_type(), right_meta_data.expression_type());

                if left_type != right_type {
                    let validation_error = ValidationError::CompareTypeMissmatch {
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                        location: node.location().to_owned(),
                    };

                    let validation_error = meta_data.set_error(validation_error.into());

                    if let Some(validation_error) = validation_error {
                        errors.insert(validation_error);
                    }
                }

                meta_data.subject = Some(left);
                meta_data.equality = Some(right_type);
                meta_data.adopt_errors(left_meta_data);
                meta_data.adopt_errors(right_meta_data);

                data_map.insert(node, meta_data);
            },

            AstEvent::Conditional { .. } => {},
            AstEvent::AlternateBody { .. } => {},
            AstEvent::AfterIf { .. } => {},
            AstEvent::ConsequentBody { test, statement } => {
                let mut meta_data = MetaCarry::new();
                let mut block_scope = Scope::from(&*scope);

                let test_meta = data_map.get(&test).expect("we should have meta data!");

                clear_error(test_meta, errors);

                if let Some(eq_type) = &test_meta.equality {
                    let tested_node = test_meta.subject.as_ref().expect("if a equality test exists there is a subject!");
                    let tested_meta = data_map.get(&tested_node).expect("there must be meta data for a tested node!");

                    if let Some(prop_access) = &tested_meta.property_access {
                        let object_meta = data_map.get(prop_access.object).expect("we have a object so there should be meta data");
                        let object_type = object_meta.expression_type();
                        let property_name = prop_access.property.source();
                        let location = test.location();

                        match object_type {
                            Type::Object(inner_type) => {
                                let custom_type = cast_ref::<MutexRef<ObjectType>, MutexRef<dyn CustomType>>(&inner_type)
                                                        .expect("we should be able to cast from ObjectType to CustomType");

                                let custom_type = block_scope.bind(custom_type);

                                custom_type.borrow_mut_safe(|object_type| {
                                    let unboxed_object_type = &mut **object_type;

                                    update_object_property_type(
                                        unboxed_object_type,
                                        &property_name,
                                        eq_type,
                                        location
                                    );
                                })
                            },

                            Type::Function(inner_type) => {
                                let custom_type = cast_ref::<MutexRef<FunctionType>, MutexRef<dyn CustomType>>(&inner_type)
                                                        .expect("we should be able to cast from ObjectType to CustomType");

                                let custom_type = block_scope.bind(custom_type);

                                custom_type.borrow_mut_safe(|object_type| {
                                    let unboxed_object_type = &mut **object_type;

                                    update_object_property_type(
                                        unboxed_object_type,
                                        &property_name,
                                        eq_type,
                                        location
                                    );
                                })
                            },

                            _ => unreachable!("it has to be a object variant")
                        };
                    }

                    if tested_meta.variable.is_none() {
                        let error = ValidationError::NonsensicalComparison {
                            expression: tested_node.source(),
                            location: tested_node.location().clone(),
                        };

                        let error = meta_data.set_error(Arc::from(error));

                        if let Some(error) = error {
                            errors.insert(error);
                        }
                    }

                    tested_meta.variable.as_ref().unwrap().borrow_mut_safe(|variable| {
                        variable.change(TracedTypeChange, eq_type.clone(), test.location().clone())
                    });
                }

                block_scope.set_name("IfConsequentBlockScope".to_string());

                let local_event_record = travel_ast_statement(statement);

                validate_events(local_event_record, data_map, scope, errors)
            },
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

                    let error = meta_data.set_error(Arc::from(error));

                    if let Some(error) = error {
                        errors.insert(error);
                    }
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
}

fn update_object_property_type<T: TracedChange<TracedTypeMuation, Type, Location> + ?Sized>(object: &mut T, property: &str, new_type: &Type, location: &Location) {
    object.change(TracedTypeMuation::Update(property.to_owned()), new_type.clone(), location.clone())
}

fn clear_error(meta: &MetaCarry, errors: &mut ErrorVec) {
    if meta.error().is_none() {
        return;
    }

    errors.remove(meta.error().as_ref().unwrap());
}
