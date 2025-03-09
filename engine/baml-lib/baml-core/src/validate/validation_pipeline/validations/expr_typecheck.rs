use anyhow::Result;
use std::collections::HashMap;

use crate::ir::repr::{initial_context, ExprMetadata};
use crate::ir::IntermediateRepr;
use crate::validate::validation_pipeline::context::Context;
use crate::Configuration;
use baml_types::expr::{Arrow, Expr, ExprType};
use baml_types::{BamlValueWithMeta, FieldType};
use internal_baml_diagnostics::{DatamodelError, Diagnostics, Span};

use crate::ir::IRHelper;

pub fn typecheck_exprs(ctx: &mut Context<'_>) -> Result<()> {
    let null_configuration = Configuration::new();
    if let Ok(ir) = IntermediateRepr::from_parser_database(ctx.db, null_configuration) {
        let typing_context: HashMap<String, ExprType> = HashMap::new();

        for expr_fn in ir.expr_fns.iter() {
            typecheck_in_context(
                &ir,
                &mut ctx.diagnostics,
                &typing_context,
                &expr_fn.elem.expr,
            )?;
        }
    }
    Ok(())
}

pub fn typecheck_in_context<U: Clone + std::fmt::Debug>(
    ir: &IntermediateRepr,
    diagnostics: &mut Diagnostics,
    typing_context: &HashMap<String, ExprType>,
    expr: &Expr<ExprMetadata, U>,
) -> Result<()> {
    eprintln!(
        "\ntypecheck: ({}): {}",
        expr.dump_str(),
        expr.meta()
            .1
            .as_ref()
            .map_or("?".to_string(), |t| t.dump_str())
    );
    for (k, v) in typing_context {
        eprintln!("  {} -> {:?}", k, v.dump_str());
    }
    match expr {
        Expr::Atom(atom, maybe_type) => {
            // Atoms always typecheck.
            Ok(())
        }
        Expr::LLMFunction(llm_function, args, _) => {
            // Bare functions always typecheck.
            Ok(())
        }
        Expr::Var(var, maybe_type) => {
            if let (span, Some(ExprType::Atom(var_type))) = maybe_type {
                if let Some(ExprType::Atom(ctx_type)) = typing_context.get(var) {
                    if ir.is_subtype(&ctx_type, var_type) {
                        Ok(())
                    } else {
                        diagnostics.push_error(DatamodelError::new_validation_error(
                            "Type mismatch",
                            span.clone(),
                        ));
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        }
        Expr::Lambda(param_names, body, (span, maybe_type)) => {
            // (\(x,y) -> x + y) : (Int,Int) -> Int
            if let Some(ExprType::Arrow(arrow)) = maybe_type {
                let mut inner_context = typing_context.clone();
                for (param_type, param_name) in arrow.param_types.iter().zip(param_names.iter()) {
                    eprintln!("inserting {:?} -> {:?}", param_name, param_type);
                    inner_context.insert(param_name.to_string(), param_type.clone());
                }
                if !compatible_as_subtype(ir, &body.meta().1, &Some(arrow.body_type.clone())) {
                    diagnostics.push_error(DatamodelError::new_validation_error(
                        &format!(
                            "Type mismatch in lambda: {:?} vs {:?}",
                            body.meta().1,
                            arrow.body_type
                        ),
                        span.clone(),
                    ));
                } else {
                    eprintln!(
                        "Type MATCH in lambda: {:?} vs {:?}",
                        body.meta().1,
                        arrow.body_type
                    );
                }
                typecheck_in_context(ir, diagnostics, &inner_context, body)?;
            }
            Ok(())
        }
        // (\[x,y] -> x + y) (1,2)
        // ([Int,Int] -> Int) ([Int,Int]
        Expr::App(f, xs, (span, maybe_type)) => {
            match (f.as_ref(), xs.as_ref(), maybe_type) {
                (
                    Expr::Lambda(params, body, (lambda_span, maybe_lambda_type)),
                    Expr::ArgsTuple(args, (args_span, args_type)),
                    Some(app_type),
                ) => {
                    // First, check that the arguments are the right type
                    // for the lambda.
                    if let Some(lambda_type) = maybe_lambda_type {
                        eprintln!("checking lambda_type: {:?}", lambda_type);
                        match lambda_type {
                            ExprType::Arrow(arrow) => {
                                if !compatible_as_subtype(
                                    ir,
                                    &Some(app_type.clone()),
                                    &Some(arrow.body_type.clone()),
                                ) {
                                    eprintln!(
                                        "Type mismatch in app: {:?} vs {:?}",
                                        app_type, arrow.body_type
                                    );
                                    diagnostics.push_error(DatamodelError::new_validation_error(
                                        &format!(
                                            "Type mismatch in app: {:?} vs {:?}",
                                            app_type, arrow.body_type
                                        ),
                                        span.clone(),
                                    ));
                                }
                                for (param_type, arg) in arrow.param_types.iter().zip(args.iter()) {
                                    if !compatible_as_subtype(
                                        ir,
                                        &arg.meta().1,
                                        &Some(param_type.clone()),
                                    ) {
                                        eprintln!(
                                            "Type mismatch in app: {:?} vs {:?}",
                                            arg.meta().1,
                                            param_type
                                        );
                                        diagnostics.push_error(
                                            DatamodelError::new_validation_error(
                                                &format!(
                                                    "Type mismatch in app: {:?} vs {:?}",
                                                    arg.meta().1,
                                                    param_type
                                                ),
                                                span.clone(),
                                            ),
                                        );
                                    }
                                }
                            }
                            ExprType::Atom(_) => {
                                diagnostics.push_error(DatamodelError::new_validation_error(
                                    "Expected a function type",
                                    span.clone(),
                                ));
                            }
                        }
                    }

                    // Then, check the body.
                    let mut inner_context = typing_context.clone();
                    for (param, arg) in params.iter().zip(args.iter()) {
                        if let Some(ref arg_type) = arg.meta().1 {
                            eprintln!("inserting {:?} -> {:?}", param, arg_type);
                            inner_context.insert(param.to_string(), arg_type.clone());
                        }
                    }
                    typecheck_in_context(ir, diagnostics, &inner_context, body)?;

                    Ok(())
                }
                _ => Ok(()),
            }
            // Applications typecheck if the function arguments
        }
        Expr::Let(let_expr, _, _, _) => Ok(()),
        Expr::ArgsTuple(args, _) => Ok(()),
    }
}

fn is_subtype(ir: &IntermediateRepr, a: &ExprType, b: &ExprType) -> bool {
    match (a, b) {
        (ExprType::Atom(a), ExprType::Atom(b)) => ir.is_subtype(a, b),
        (ExprType::Arrow(a), ExprType::Arrow(b)) => {
            let a_arrow = a.as_ref();
            let b_arrow = b.as_ref();
            let return_type_ok = is_subtype(ir, &a_arrow.body_type, &b_arrow.body_type);
            let arg_types_ok = a_arrow
                .param_types
                .iter()
                .zip(b_arrow.param_types.iter())
                .all(|(a, b)| is_subtype(ir, b, a));
            return_type_ok && arg_types_ok
        }
        _ => false,
    }
}

fn compatible_as_subtype(
    ir: &IntermediateRepr,
    a: &Option<ExprType>,
    b: &Option<ExprType>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => is_subtype(ir, a, b),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::repr::make_test_ir_and_diagnostics;

    #[test]
    fn null_case() {
        let (ir, mut diagnostics) = make_test_ir_and_diagnostics(
            r##"
        fn First(x: int, y: int) -> string {
          x
        }
        "##,
        )
        .expect("Valid source");
        assert!(diagnostics.has_errors());
    }
}
