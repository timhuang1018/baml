use anyhow::Result;
use std::collections::HashMap;

use baml_types::{BamlValueWithMeta, FieldType};
use baml_types::expr::{Expr, ExprType};
use crate::ir::repr::{initial_context, ExprMetadata};
use crate::ir::IntermediateRepr;
use crate::validate::validation_pipeline::context::Context;
use crate::Configuration;
use internal_baml_diagnostics::{DatamodelError, Diagnostics, Span};

use crate::ir::IRHelper;

pub fn typecheck_exprs(ctx: &mut Context<'_>) -> Result<()> {
    let null_configuration = Configuration::new();
    let ir = IntermediateRepr::from_parser_database(ctx.db, null_configuration)?;
    let typing_context: HashMap<String, ExprType> = HashMap::new();
    // let value_context = initial_context(&ir);

    for expr_fn in ir.expr_fns.iter() {
        typecheck_in_context(&ir, &mut ctx.diagnostics, &typing_context, &expr_fn.elem.body)?;
    }
    Ok(())
}


pub fn typecheck_in_context<U: Clone + std::fmt::Debug>(
    ir: &IntermediateRepr,
    diagnostics: &mut Diagnostics,
    typing_context: &HashMap<String, ExprType>,
    expr: &Expr<ExprMetadata,U>,
) -> Result<()> {
    match expr {
        Expr::Atom(atom, maybe_type) => {
            // Atoms always typecheck.
            Ok(())
        },
        Expr::LLMFunction(llm_function, args, _) => {
            // Bare functions always typecheck.
            Ok(())
        },
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
        },
        Expr::Lambda(lambda, args, maybe_type) => {
            // A lambda typechecks by infering from its parameters
            // and then typechecking the body.
            // We can't infer the parameters yet.
                Ok(())
        },
        Expr::App(f, xs, maybe_type) => {
            match (f, xs, maybe_type) {
                (Expr::Lambda(params, body, lambda_type), Expr::ArgsTuple(argrs, args_type), Some(app_type)) => {

                }
                _ => todo!()
            }
            // Applications typecheck if the function arguments 
            Ok(())
        },
        Expr::Let(let_expr, _, _, _) => { Ok(()) },
        Expr::ArgsTuple(args, _) => { Ok(()) },
    }
}