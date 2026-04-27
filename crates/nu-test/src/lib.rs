use std::{
    borrow::Cow,
    env,
    num::NonZeroUsize,
    path::{Component, Path},
    sync::Arc,
};

use itertools::Itertools;
use kitest::runner::DefaultRunner;
use nu_protocol::{
    Config, IntoValue, Span, Value,
    engine::{EngineState, StateWorkingSet},
};

use crate::group::{ModuleRunner, TestModules};

pub mod discover;
pub mod group;
pub mod test;

pub fn engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    let mut engine_state = nu_cli::add_cli_context(engine_state);

    let mut working_set = StateWorkingSet::new(&engine_state);
    working_set.add_decl(Box::new(nu_cli::Print));
    engine_state.merge_delta(working_set.delta).unwrap();

    engine_state.generate_nu_constant();
    [
        (
            "PWD",
            Value::test_string(env::current_dir().unwrap().display().to_string()),
        ),
        ("config", Config::default().into_value(Span::unknown())),
    ]
    .into_iter()
    .for_each(|(key, val)| engine_state.add_env_var(key.into(), val));

    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
}

#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleName(Arc<str>);

pub fn module_name(cwd: impl AsRef<Path>, module_path: impl AsRef<Path>) -> ModuleName {
    let module_path = module_path.as_ref();
    let relative_path = pathdiff::diff_paths(module_path, cwd);
    let components = relative_path
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or(module_path)
        .components();

    let mut module_name = components
        .into_iter()
        .flat_map(|c| match c {
            Component::Normal(name) if name == "mod.nu" => None,
            Component::Normal(name) => name
                .to_str()
                .map(|s| Cow::Borrowed(s))
                .unwrap_or_else(|| name.to_string_lossy().into())
                .into(),
            _ => None,
        })
        .join("/");

    if module_name.ends_with(".nu") {
        module_name.truncate(module_name.len() - const { ".nu".len() });
    }

    ModuleName(Arc::from(module_name))
}

#[non_exhaustive]
#[derive(Default, Debug)]
pub struct Args {}

pub fn run_test_harness(
    initial_engine_state: &EngineState,
    test_path: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
    args: Args,
) -> miette::Result<()> {
    let cwd = cwd.as_ref().canonicalize().unwrap();
    let test_path = cwd.join(test_path).canonicalize().unwrap();
    let discoveries = discover::discover_recursively(&initial_engine_state, test_path).unwrap();
    let test_iter = discoveries
        .into_iter()
        .map(|discovery| test::build_tests(discovery, &cwd));
    let mut all_tests = Vec::new();
    let mut test_modules = TestModules::new();
    for (tests, test_module) in test_iter.into_iter().filter(|(tests, _)| tests.len() > 0) {
        all_tests.extend(tests);
        test_modules.insert(
            all_tests
                .last()
                .expect("not empty")
                .extra
                .module_name
                .clone(),
            test_module,
        );
    }

    let _ = kitest::harness(&all_tests)
        .with_grouper(test_modules)
        .with_runner(DefaultRunner::default().with_thread_count(NonZeroUsize::MIN))
        .with_group_runner(ModuleRunner)
        .run();

    Ok(())
}
