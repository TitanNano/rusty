use ratel::{ ast as Ast };
use toolshed::Arena;
use std::ops::{ Deref, DerefMut };
use owner::Owner;

pub enum HookType<'own, 're, T: PiggybackCapable + 'own> {
    Assignment {
        ident: &'re Backpack<'own, T>,
        receiver: Backpack<'own, T>,
        value: Backpack<'own, T>,
    },

    Addition {
        ident: &'re Backpack<'own, T>,
        receiver: Backpack<'own, T>,
        value: Backpack<'own, T>,
    },

    Equality {
        ident: &'re Backpack<'own, T>,
        left: Backpack<'own, T>,
        right: Backpack<'own, T>,
    },

    StrictInequality {
        ident: &'re Backpack<'own, T>,
        left: Backpack<'own, T>,
        right: Backpack<'own, T>,
    },

    Conditional {
        test: &'re Backpack<'own, T>,
        consequent: Backpack<'own, T>,
        alternate: Backpack<'own, T>,
    },

    PropertyAccess {
        ident: &'re Backpack<'own, T>,
        object: Backpack<'own, T>,
        property: Ast::IdentifierNode<'own>,
    },

    DynamicPropertyAccess {
        ident: &'re Backpack<'own, T>,
        object: Backpack<'own, T>,
        property: Backpack<'own, T>,
    },

    ConsequentBody {
        test: &'re Backpack<'own, T>,
        consequent: Ast::StatementNode<'own>,
    },

    AlternateBody {
        test: &'re Backpack<'own, T>,
        alternate: Ast::StatementNode<'own>,
    },

    AfterIf {
        test: &'re Backpack<'own, T>,
    },

    Identifier {
        node: &'re Backpack<'own, T>,
        identifier: &'re str,
    },

    Literal {
        ident: &'re Backpack<'own, T>,
        literal: Ast::Literal<'own>,
    },

    Array {
        ident: &'re Backpack<'own, T>,
        expression: Ast::expression::ArrayExpression<'own>,
    },

    Sequence {
        ident: &'re Backpack<'own, T>,
        sequence: Ast::ExpressionList<'own>,
    },

    This {
        ident: &'re Backpack<'own, T>,
    },

    PreOrPostFix {
        ident: &'re Backpack<'own, T>,
        operator: Ast::OperatorKind,
        operand: Backpack<'own, T>
    },

    Template {
        ident: &'re Backpack<'own, T>
    },

    FunctionCall {
        ident: &'re Backpack<'own, T>,
        function: Backpack<'own, T>,
        arguments: Vec<Backpack<'own, T>>,
    },

    Spread {
        ident: &'re Backpack<'own, T>,
        item: Backpack<'own, T>,
    },

    Object {
        ident: &'re Backpack<'own, T>,
        body: Ast::NodeList<'own, Ast::Property<'own>>,
    },

    Function {
        ident: &'re Backpack<'own, T>,
        params: Ast::PatternList<'own>,
        body: &'re Ast::Node<'own, Ast::BlockStatement<'own>>,
        arrow: bool,
    },

    Class {
        ident: &'re Backpack<'own, T>,
        class: Ast::expression::ClassExpression<'own>
    }
}


pub struct Backpack<'own, T: PiggybackCapable + 'own> {
    own: Ast::ExpressionNode<'own>,
    carried: T,
}

impl<'own, T: PiggybackCapable> Backpack<'own, T> {
    fn update(&mut self, hook_data: Ast::ExpressionNode<'own>) {
        self.own = hook_data;
    }

    fn carry(&mut self, data: T) {
        self.carried = data;
    }

    fn new(own: Ast::ExpressionNode<'own>) -> Self {
        Self { own, carried: T::new() }
    }
}

impl<'own, T: PiggybackCapable> Deref for Backpack<'own, T> {
    type Target = Ast::ExpressionNode<'own>;

    fn deref(&self) -> &Self::Target {
        &self.own
    }
}

impl<'own, T: PiggybackCapable> DerefMut for Backpack<'own, T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.own
    }
}

pub trait PiggybackCapable {
    fn new() -> Self;
}


pub fn travel_ast<'a, 'b, T: PiggybackCapable + 'a, Func: FnMut(&HookType<T>)>(ast: Ast::StatementList<'a>, mut callback: Func) {
    for statement in ast {
        match statement.item {
            Ast::Statement::Expression(expression) => {
                travel_expression(expression, &mut callback);
            },

            Ast::Statement::If(if_statement) => {
                let test = travel_expression(if_statement.test, &mut callback);

                callback(&HookType::ConsequentBody { test: &test, consequent: if_statement.consequent });

                if let Some(alternate) = if_statement.alternate {
                    callback(&HookType::AlternateBody { test: &test, alternate });
                }

                callback(&HookType::AfterIf { test: &test });
            }

            _ => (),
        }
    }
}

fn travel_expression<'local_ast, 'b, T: PiggybackCapable + 'local_ast>(expression: Ast::ExpressionNode<'local_ast>, callback: &mut impl FnMut(&HookType<T>)) -> Backpack<'local_ast, T> {

    match expression.item {
        Ast::Expression::Binary(binary_expression) => {
            let operator = binary_expression.operator;
            let left = travel_expression(binary_expression.left, callback);
            let right = travel_expression(binary_expression.right, callback);
            let ident = Backpack::new(expression);

            match operator {
                Ast::OperatorKind::Assign => {
                    callback(&HookType::Assignment { ident: &ident, receiver: left, value: right, });
                },

                Ast::OperatorKind::Addition => {
                    callback(&HookType::Addition { ident: &ident, receiver: left, value: right, });
                },

                Ast::OperatorKind::StrictEquality => {
                    callback(&HookType::Equality { ident: &ident, left, right });
                }

                Ast::OperatorKind::StrictInequality => {
                    callback(&HookType::StrictInequality { ident: &ident, left, right });
                }

                _ => (),
            };

            ident
        },

        Ast::Expression::Conditional(conditional_expression) => {
            let test = travel_expression(conditional_expression.test, callback);
            let consequent = travel_expression(conditional_expression.consequent, callback);
            let alternate = travel_expression(conditional_expression.consequent, callback);

            callback(&HookType::Conditional { test: &test, consequent, alternate });

            test
        },

        Ast::Expression::Member(member_expression) => {
            let object = travel_expression(member_expression.object, callback);
            let property = member_expression.property;
            let ident = Backpack::new(expression);

            callback(&HookType::PropertyAccess { ident: &ident, object, property: property });

            ident
        }

        Ast::Expression::Identifier(identifier_expression) => {
            let node = Backpack::new(expression);

            callback(&HookType::Identifier { node: &node, identifier: identifier_expression });

            node
        },

        Ast::Expression::Void => {
            Backpack::new(expression)
        },

        Ast::Expression::Literal(literal_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::Literal { ident: &ident, literal: literal_expression });

            ident
        },

        Ast::Expression::Array(array_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::Array { ident: &ident, expression: array_expression });

            ident
        },

        Ast::Expression::Sequence(sequence_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::Sequence { ident: &ident, sequence: sequence_expression.body });

            ident
        },

        Ast::Expression::This(this_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::This { ident: &ident, });

            ident
        },

        Ast::Expression::MetaProperty(meta_property) => {
            panic!("got meta property access {}.{}", **meta_property.meta, **meta_property.property);
        },

        Ast::Expression::ComputedMember(computed_member) => {
            let ident = Backpack::new(expression);
            let object = travel_expression(computed_member.object, callback);
            let property = travel_expression(computed_member.property, callback);

            callback(&HookType::DynamicPropertyAccess { ident: &ident, object, property });

            ident
        },

        Ast::Expression::Call(function_call) => {
            let ident = Backpack::new(expression);
            let function = travel_expression(function_call.callee, callback);
            let arguments: Vec<Backpack<T>> = function_call.arguments.iter().map(|argument| travel_expression(*argument, callback)).collect();

            callback(&HookType::FunctionCall { ident: &ident, function, arguments });

            ident
        },

        Ast::Expression::Prefix(prefix) => {
            let ident = Backpack::new(expression);
            let operand = travel_expression(prefix.operand, callback);
            let operator = prefix.operator;

            callback(&HookType::PreOrPostFix { ident: &ident, operand, operator });

            ident
        },

        Ast::Expression::Postfix(postfix) => {
            let ident = Backpack::new(expression);
            let operand = travel_expression(postfix.operand, callback);
            let operator = postfix.operator;

            callback(&HookType::PreOrPostFix { ident: &ident, operand, operator });

            ident
        }

        Ast::Expression::Template(template) => {
            let ident = Backpack::new(expression);

            // we are currently ignoring all expressions inside the template literal
            callback(&HookType::Template { ident: &ident });

            ident
        },

        Ast::Expression::TaggedTemplate(template) => {
            let ident = Backpack::new(expression);
            let function = travel_expression(template.tag, callback);
            let template_expression = Ast::Expression::Template(**template.quasi);
            let loc = Ast::Loc::new(template.quasi.start, template.quasi.end, template_expression);
            let arguments = vec!(travel_expression(Ast::ExpressionNode::new(&loc), callback));

            callback(&HookType::FunctionCall { ident: &ident, function, arguments });

            ident
        },

        Ast::Expression::Spread(spread_expression) => {
            let ident = Backpack::new(expression);
            let item = travel_expression(spread_expression.argument, callback);

            callback(&HookType::Spread { ident: &ident, item });

            ident
        },

        Ast::Expression::Object(object_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::Object { ident: &ident, body: object_expression.body });

            ident
        },

        Ast::Expression::Arrow(arrow_function) => {
            let ident = Backpack::new(expression);
            let arena = Arena::new();
            let Ast::expression::ArrowExpression { params, body, .. } = arrow_function;

            let body = match body {
                Ast::expression::ArrowBody::Block(block) => { block },
                Ast::expression::ArrowBody::Expression(expression) => {
                    let return_statement = Ast::Statement::Return(Ast::statement::ReturnStatement {
                        value: Some(expression)
                    });
                    let return_statement_loc = Ast::Loc::new(expression.start, expression.end, return_statement);
                    let return_statement_loc = &*arena.alloc(return_statement_loc);
                    let return_statement_node = Ast::Node::new(&return_statement_loc);

                    let block_statement = Ast::Block {
                        body: Ast::NodeList::from(&arena, return_statement_node)
                    };

                    let block_statement_loc = Ast::Loc::new(expression.start, expression.end, block_statement);
                    let block_statement_loc = arena.alloc(block_statement_loc);

                    let node = Ast::Node::new(&block_statement_loc);

                    node
                },

            };

//            callback(&HookType::Function { ident: &ident, params, body: &body, arrow: true });

            ident
        },

        Ast::Expression::Function(function) => {
            let ident = Backpack::new(expression);
            let Ast::Function { params, body, .. } = function;

            callback(&HookType::Function { ident: &ident, params, body: &body, arrow: false });

            ident
        },

        Ast::Expression::Class(class_expression) => {
            let ident = Backpack::new(expression);

            callback(&HookType::Class { ident: &ident, class: class_expression });

            ident
        }

    }
}
