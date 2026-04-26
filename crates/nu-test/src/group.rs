use std::{
    collections::HashMap,
    fmt::{self, Display},
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::Arc,
};

use kitest::{
    group::{TestGroupOutcomes, TestGroupRunner, TestGrouper},
    test::TestMeta,
};
use nu_protocol::{
    BlockId, PipelineData,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};

use crate::{ModuleName, test::Extra};

#[derive(Debug, Clone, Default)]
pub struct TestModules(HashMap<ModuleName, TestModule>);

impl TestModules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<ModuleName>, module: TestModule) {
        self.0.insert(key.into(), module);
    }
}

impl TestGrouper<Extra, ModuleName, TestModule> for TestModules {
    fn group(&mut self, meta: &TestMeta<Extra>) -> ModuleName {
        // the test modules are inserted before hand to avoid unnecessary bloat in Extra
        meta.extra.module_name.clone()
    }

    fn group_ctx(&mut self, key: &ModuleName) -> Option<TestModule> {
        self.0.remove(key)
    }
}

#[derive(Clone, derive_more::Debug)]
pub struct TestModule {
    #[debug("EngineState {{...}}")]
    pub engine_state: EngineState,
    pub before_all_block_ids: Vec<BlockId>,
    pub after_all_block_ids: Vec<BlockId>,
}

pub struct ModuleRunner;

impl<'t> TestGroupRunner<'t, Extra, ModuleName, TestModule> for ModuleRunner {
    fn run_group<F>(
        &self,
        f: F,
        _: &ModuleName,
        test_module: Option<&TestModule>,
    ) -> ControlFlow<TestGroupOutcomes<'t>, TestGroupOutcomes<'t>>
    where
        F: FnOnce() -> TestGroupOutcomes<'t>,
    {
        if let Some(test_module) = test_module {
            for block_id in test_module.before_all_block_ids.iter().copied() {
                let mut stack = Stack::new();
                let block = test_module.engine_state.get_block(block_id);
                // TODO: do something with the error here
                nu_engine::eval_block::<WithoutDebug>(
                    &test_module.engine_state,
                    &mut stack,
                    block,
                    PipelineData::empty(),
                )
                .unwrap();
            }
        }

        let res = f();

        // TODO: deduplicate this
        if let Some(test_module) = test_module {
            for block_id in test_module.after_all_block_ids.iter().copied() {
                let mut stack = Stack::new();
                let block = test_module.engine_state.get_block(block_id);
                // TODO: do something with the error here
                nu_engine::eval_block::<WithoutDebug>(
                    &test_module.engine_state,
                    &mut stack,
                    block,
                    PipelineData::empty(),
                )
                .unwrap();
            }
        }

        ControlFlow::Continue(res)
    }
}
