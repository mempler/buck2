/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::sync::Arc;

use allocative::Allocative;
use async_trait::async_trait;
use buck2_common::dice::cells::HasCellResolver;
use buck2_common::result::SharedResult;
use buck2_common::result::ToSharedResultExt;
use buck2_common::result::ToUnsharedResultExt;
use buck2_core::bzl::ImportPath;
use buck2_core::cells::cell_path::CellPathRef;
use buck2_core::cells::paths::CellRelativePath;
use buck2_core::configuration::data::ConfigurationData;
use buck2_interpreter::paths::package::PackageFilePath;
use buck2_interpreter_for_build::interpreter::package_file_calculation::EvalPackageFile;
use buck2_node::cfg_constructor::CfgConstructorCalculationImpl;
use buck2_node::cfg_constructor::CfgConstructorImpl;
use buck2_node::cfg_constructor::CFG_CONSTRUCTOR_CALCULATION_IMPL;
use derive_more::Display;
use dice::CancellationContext;
use dice::DiceComputations;
use dice::Key;
use dupe::Dupe;
use dupe::OptionDupedExt;

pub struct CfgConstructorCalculationInstance;

async fn get_cfg_constructor_uncached(
    ctx: &DiceComputations,
) -> anyhow::Result<Option<Arc<dyn CfgConstructorImpl>>> {
    let root_cell = ctx.get_cell_resolver().await?.root_cell();
    let package_file_path =
        PackageFilePath::for_dir(CellPathRef::new(root_cell, CellRelativePath::empty()));
    // This returns empty super package if `PACKAGE` file does not exist.
    let super_package = ctx.eval_package_file(&package_file_path).await?;
    Ok(super_package.cfg_constructor().duped())
}

#[async_trait]
impl CfgConstructorCalculationImpl for CfgConstructorCalculationInstance {
    async fn get_cfg_constructor(
        &self,
        ctx: &DiceComputations,
    ) -> anyhow::Result<Option<Arc<dyn CfgConstructorImpl>>> {
        #[derive(Clone, Dupe, Display, Debug, Eq, Hash, PartialEq, Allocative)]
        struct GetCfgConstructorKey;

        #[async_trait]
        impl Key for GetCfgConstructorKey {
            type Value = SharedResult<Option<Arc<dyn CfgConstructorImpl>>>;

            async fn compute(
                &self,
                ctx: &mut DiceComputations,
                _cancellations: &CancellationContext,
            ) -> Self::Value {
                get_cfg_constructor_uncached(ctx).await.shared_error()
            }

            fn equality(_x: &Self::Value, _y: &Self::Value) -> bool {
                false
            }
        }

        ctx.compute(&GetCfgConstructorKey).await?.unshared_error()
    }

    async fn eval_cfg_constructor(
        &self,
        ctx: &DiceComputations,
        cfg: ConfigurationData,
    ) -> anyhow::Result<ConfigurationData> {
        #[derive(Clone, Dupe, Display, Debug, Eq, Hash, PartialEq, Allocative)]
        struct CfgConstructorInvocationKey {
            cfg: ConfigurationData,
        }

        #[async_trait]
        impl Key for CfgConstructorInvocationKey {
            type Value = SharedResult<ConfigurationData>;

            async fn compute(
                &self,
                ctx: &mut DiceComputations,
                _cancellations: &CancellationContext,
            ) -> Self::Value {
                // Invoke eval fn from global instance of cfg constructors
                match CFG_CONSTRUCTOR_CALCULATION_IMPL
                    .get()?
                    .get_cfg_constructor(ctx)
                    .await?
                {
                    Some(cfg_constructor) => {
                        cfg_constructor.eval(ctx, &self.cfg).await.shared_error()
                    }
                    // By this point we should have already confirmed that the global cfg constructor instance exists
                    None => unreachable!("Global cfg constructor instance should exist."),
                }
            }

            fn equality(x: &Self::Value, y: &Self::Value) -> bool {
                match (x, y) {
                    (Ok(x), Ok(y)) => x == y,
                    _ => false,
                }
            }
        }

        match self.get_cfg_constructor(ctx).await? {
            Some(_) => {
                let key = CfgConstructorInvocationKey { cfg };
                Ok(ctx.compute(&key).await??)
            }
            // To facilitate rollout of modifiers, return original configuration if
            // no cfg constructors are available.
            None => Ok(cfg),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CfgConstructorLocation {
    pub import_path: ImportPath,
    pub function: String,
}
