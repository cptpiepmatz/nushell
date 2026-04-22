use std::{
    collections::HashMap,
    mem,
    path::{PathBuf, Path}
};

use nu_protocol::{
    BlockId, CompileError, ParseError, ShellError, Value,
    engine::{EngineState, StateWorkingSet},
};
use thiserror::Error;

#[derive(derive_more::Debug)]
pub struct Discovery {
    pub path: PathBuf,
    #[debug("EngineState {{...}}")]
    pub engine_state: EngineState,
    pub tests: Vec<DiscoveredTest>,
    pub before_each: Vec<DiscoveredLifecycleHook>,
    pub after_each: Vec<DiscoveredLifecycleHook>,
    pub before_all: Vec<DiscoveredLifecycleHook>,
    pub after_all: Vec<DiscoveredLifecycleHook>,
}

#[derive(Debug)]
pub struct DiscoveredTest {
    pub block_id: BlockId,
    pub name: String,
    pub ignore: Option<Value>,
    pub test_value: Value,
    // TODO: add more relevant fields
}

#[derive(Debug)]
pub struct DiscoveredLifecycleHook {
    pub block_id: BlockId,
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Error)]
pub enum DiscoverError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Compile(#[from] CompileError),

    #[error(transparent)]
    MergeDelta(#[from] ShellError),
}

pub fn discover(
    mut engine_state: EngineState,
    path: impl AsRef<Path>,
) -> Result<Discovery, DiscoverError> {
    let mut working_set = StateWorkingSet::new(&engine_state);
    let code = format!("overlay new testing; source '{}'", path.as_ref().display());
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);

    mem::take(&mut working_set.parse_errors)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)?;
    mem::take(&mut working_set.compile_errors)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)?;

    engine_state.merge_delta(working_set.delta)?;

    let commands = engine_state
        .scope
        .overlays
        .iter()
        .skip(1) // skip first overlay which is 'zero'
        .map(|(_, overlay)| overlay.decls.values())
        .flatten()
        .map(|decl_id| engine_state.get_decl(*decl_id))
        .flat_map(|command| command.block_id().map(|block_id| (command, block_id)));

    let mut tests = Vec::new();
    let mut before_each = Vec::new();
    let mut after_each = Vec::new();
    let mut before_all = Vec::new();
    let mut after_all = Vec::new();

    for (command, block_id) in commands {
        let name = command.name();
        let mut attributes: HashMap<_, _> = command.attributes().into_iter().collect();

        if let Some(test_value) = attributes.remove("test") {
            tests.push(DiscoveredTest {
                block_id,
                name: name.to_string(),
                ignore: attributes.remove("ignore"),
                test_value,
            });
        }

        if let Some(value) = attributes.remove("before-each") {
            before_each.push(DiscoveredLifecycleHook {
                block_id,
                name: name.to_string(),
                value,
            })
        }

        if let Some(value) = attributes.remove("after-each") {
            after_each.push(DiscoveredLifecycleHook {
                block_id,
                name: name.to_string(),
                value,
            })
        }

        if let Some(value) = attributes.remove("before-all") {
            before_all.push(DiscoveredLifecycleHook {
                block_id,
                name: name.to_string(),
                value,
            })
        }

        if let Some(value) = attributes.remove("after-all") {
            after_all.push(DiscoveredLifecycleHook {
                block_id,
                name: name.to_string(),
                value,
            })
        }
    }

    Ok(Discovery {
        path: path.as_ref().to_path_buf(),
        engine_state,
        tests,
        before_each,
        after_each,
        before_all,
        after_all,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_test() {
        let engine_state = crate::engine_state();
        let discovery = discover(engine_state, "tests/example.nu").unwrap();
        dbg!(discovery);

        // todo!()
    }
}
