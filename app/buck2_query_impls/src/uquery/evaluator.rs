/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Implementation of the cli and query_* attr query language.
use std::sync::Arc;

use buck2_core::fs::project_rel_path::ProjectRelativePath;
use buck2_core::target::label::TargetLabel;
use buck2_node::nodes::unconfigured::TargetNode;
use buck2_query::query::syntax::simple::eval::values::QueryEvaluationResult;
use buck2_query::query::syntax::simple::functions::DefaultQueryFunctionsModule;
use dice::DiceComputations;

use crate::analysis::evaluator::eval_query;
use crate::dice::get_dice_query_delegate;
use crate::dice::DiceQueryDelegate;
use crate::uquery::environment::PreresolvedQueryLiterals;
use crate::uquery::environment::UqueryEnvironment;

pub struct UqueryEvaluator<'c> {
    dice_query_delegate: DiceQueryDelegate<'c>,
    functions: DefaultQueryFunctionsModule<UqueryEnvironment<'c>>,
}

impl UqueryEvaluator<'_> {
    pub async fn eval_query(
        &self,
        query: &str,
        query_args: &[String],
    ) -> anyhow::Result<QueryEvaluationResult<TargetNode>> {
        eval_query(&self.functions, query, query_args, async move |literals| {
            let resolved_literals = PreresolvedQueryLiterals::pre_resolve(
                &**self.dice_query_delegate.query_data(),
                &literals,
                self.dice_query_delegate.ctx(),
            )
            .await;
            Ok(UqueryEnvironment::new(
                &self.dice_query_delegate,
                Arc::new(resolved_literals),
            ))
        })
        .await
    }
}

/// Evaluates some query expression. TargetNodes are resolved via the interpreter from
/// the provided DiceCtx.
pub async fn get_uquery_evaluator<'a, 'c: 'a>(
    ctx: &'c DiceComputations,
    working_dir: &'a ProjectRelativePath,
    global_target_platform: Option<TargetLabel>,
) -> anyhow::Result<UqueryEvaluator<'c>> {
    let dice_query_delegate =
        get_dice_query_delegate(ctx, working_dir, global_target_platform).await?;
    let functions = DefaultQueryFunctionsModule::new();

    Ok(UqueryEvaluator {
        dice_query_delegate,
        functions,
    })
}
