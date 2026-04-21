use std::{error::Error, fmt::{self, Display}};

use kitest::{
    Whatever,
    ignore::IgnoreStatus,
    panic::PanicExpectation,
    test::{Test, TestFn, TestFnHandle, TestMeta, TestResult},
};
use nu_protocol::{
    BlockId, PipelineData, ShellError, Value, debugger::WithoutDebug, engine::{EngineState, Stack}
};

use crate::discover::Discovery;

pub fn build_tests(discovery: Discovery) -> impl Iterator<Item = Test> {
    discovery.tests.into_iter().map(move |test| {
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
            extra: (),
        };

        Test::new(TestFnHandle::Owned(Box::new(test_fn)), meta)
    })
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
        todo!()
    }
}

impl Error for TestErrors {}

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

        if !errors.before_each_errors.is_empty()
            || errors.test_error.is_none()
            || !errors.after_each_errors.is_empty()
        {
            return TestResult(Err(Whatever::from(errors)));
        }

        TestResult(Ok(None))
    }
}
