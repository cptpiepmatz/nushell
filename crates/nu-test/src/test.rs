use std::{
    error::Error,
    fmt::{self, Display},
    path::Path,
};

use kitest::{
    Whatever,
    ignore::IgnoreStatus,
    panic::PanicExpectation,
    test::{Test, TestFn, TestFnHandle, TestMeta, TestResult},
};
use nu_protocol::{
    BlockId, PipelineData, ShellError, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};

use crate::{ModuleName, discover::Discovery, group::TestModule, module_name};

#[derive(Debug)]
pub struct Extra {
    pub module_name: ModuleName,
}

pub fn build_tests(
    discovery: Discovery,
    cwd: impl AsRef<Path>,
) -> (impl ExactSizeIterator<Item = Test<Extra>>, TestModule) {
    let module_name = module_name(cwd, discovery.path);

    let test_module = TestModule {
        engine_state: discovery.engine_state.clone(),
        before_all_block_ids: discovery
            .before_all
            .into_iter()
            .map(|hook| hook.block_id)
            .collect(),
        after_all_block_ids: discovery
            .after_all
            .into_iter()
            .map(|hook| hook.block_id)
            .collect(),
    };

    let tests = discovery.tests.into_iter().map(move |test| {
        let test_fn = NushellTestFn {
            engine_state: discovery.engine_state.clone(),
            test_block_id: test.block_id,
            before_each_block_ids: discovery
                .before_each
                .iter()
                .map(|hook| hook.block_id)
                .collect(),
            after_each_block_ids: discovery
                .after_each
                .iter()
                .map(|hook| hook.block_id)
                .collect(),
        };

        let meta = TestMeta {
            name: test.name.into(),
            ignore: match test.ignore {
                None => IgnoreStatus::Run,
                Some(Value::String { val, .. }) if val.is_empty() => IgnoreStatus::Ignore,
                Some(Value::String { val, .. }) => IgnoreStatus::IgnoreWithReason(val.into()),
                Some(_) => todo!(),
            },
            should_panic: PanicExpectation::ShouldNotPanic,
            origin: None,
            extra: Extra {
                module_name: module_name.clone(),
            },
        };

        Test::new(TestFnHandle::Owned(Box::new(test_fn)), meta)
    });

    (tests, test_module)
}

struct NushellTestFn {
    engine_state: EngineState,
    test_block_id: BlockId,
    before_each_block_ids: Vec<BlockId>,
    after_each_block_ids: Vec<BlockId>,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct TestErrors {
    before_each_errors: Vec<ShellError>,
    test_error: Option<ShellError>,
    after_each_errors: Vec<ShellError>,
}

impl Display for TestErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "no test errors");
        }

        if !self.before_each_errors.is_empty() {
            writeln!(f, "before_each hook failures:")?;
            for (i, err) in self.before_each_errors.iter().enumerate() {
                writeln!(f, "  {}. {err}", i + 1)?;
            }
        }

        if let Some(err) = &self.test_error {
            writeln!(f, "test body failure: {err}")?;
        }

        if !self.after_each_errors.is_empty() {
            writeln!(f, "after_each hook failures:")?;
            for (i, err) in self.after_each_errors.iter().enumerate() {
                writeln!(f, "  {}. {err}", i + 1)?;
            }
        }

        Ok(())
    }
}

impl Error for TestErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.primary_error().map(|e| e as &(dyn Error + 'static))
    }
}

impl TestErrors {
    fn is_empty(&self) -> bool {
        self.before_each_errors.is_empty()
            && self.test_error.is_none()
            && self.after_each_errors.is_empty()
    }

    fn primary_error(&self) -> Option<&ShellError> {
        self.test_error
            .as_ref()
            .or_else(|| self.before_each_errors.first())
            .or_else(|| self.after_each_errors.first())
    }
}

impl TestFn for NushellTestFn {
    fn call_test(&self) -> TestResult {
        let mut stack = Stack::new();
        let mut errors = TestErrors::default();

        for block_id in self.before_each_block_ids.iter().copied() {
            let block = self.engine_state.get_block(block_id);
            if let Err(err) = nu_engine::eval_block::<WithoutDebug>(
                &self.engine_state,
                &mut stack,
                block,
                PipelineData::empty(),
            ) {
                errors.before_each_errors.push(err);
            }
        }

        if errors.before_each_errors.is_empty() {
            let test_block = self.engine_state.get_block(self.test_block_id);
            if let Err(err) = nu_engine::eval_block::<WithoutDebug>(
                &self.engine_state,
                &mut stack,
                test_block,
                PipelineData::empty(),
            ) {
                errors.test_error = Some(err);
            }
        }

        for block_id in self.after_each_block_ids.iter().copied() {
            let block = self.engine_state.get_block(block_id);
            if let Err(err) = nu_engine::eval_block::<WithoutDebug>(
                &self.engine_state,
                &mut stack,
                block,
                PipelineData::empty(),
            ) {
                errors.after_each_errors.push(err);
            }
        }

        if !errors.is_empty() {
            return TestResult(Err(Whatever::from(errors)));
        }

        TestResult(Ok(None))
    }
}
