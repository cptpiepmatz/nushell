use std::{
    any::Any,
    collections::HashSet,
    fmt::{self, Debug},
    mem,
    sync::LazyLock,
};

use nu_engine::scope::ScopeData;
use nu_path::Path;
use nu_protocol::{
    CompileError, Id, ParseError, ShellError, Span,
    engine::{DEFAULT_OVERLAY_NAME, EngineState, Stack, StateWorkingSet},
};
use thiserror::Error;

pub struct Discovery {
    engine_state: EngineState,
    tests: Vec<DiscoveredTest>,
    before_each: Vec<String>,
    after_each: Vec<String>,
    before_all: Vec<String>,
    after_all: Vec<String>,
}

impl Debug for Discovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct EngineStateDebug;
        impl Debug for EngineStateDebug {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "EngineState {{...}}")
            }
        }

        f.debug_struct("Discovery")
            .field("engine_state", &EngineStateDebug)
            .field("tests", &self.tests)
            .field("before_each", &self.before_each)
            .field("after_each", &self.after_each)
            .field("before_all", &self.before_all)
            .field("after_all", &self.after_all)
            .finish()
    }
}

#[derive(Debug)]
pub struct DiscoveredTest {
    name: String,
    ignore: bool,
    // TODO: add more relevant fields
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

pub fn discover(mut engine_state: EngineState, path: impl AsRef<Path>) -> Result<Discovery, DiscoverError> {
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
        .map(|decl_id| engine_state.get_decl(*decl_id));

    let mut tests = Vec::new();
    let mut before_each = Vec::new();
    let mut after_each = Vec::new();
    let mut before_all = Vec::new();
    let mut after_all = Vec::new();

    for command in commands {
        let name = command.name();
        let attributes: HashSet<_> = command
            .attributes()
            .into_iter()
            .map(|(attr, _)| attr)
            .collect();

        if attributes.contains("test") {
            tests.push(DiscoveredTest {
                name: name.to_string(),
                ignore: attributes.contains("ignore"),
            });
        }

        let push_if_present = |attr, list: &mut Vec<_>| {
            if attributes.contains(attr) {
                list.push(name.to_string());
            }
        };
        push_if_present("before-each", &mut before_each);
        push_if_present("after-each", &mut after_each);
        push_if_present("before-all", &mut before_all);
        push_if_present("after-all", &mut after_all);
    }

    Ok(Discovery {
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

        todo!()
    }
}
