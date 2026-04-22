use std::env;

use nu_protocol::{Config, IntoValue, Span, Value, engine::{EngineState, StateWorkingSet}};

pub mod discover;
pub mod test;
pub mod group;

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
        ("PWD", Value::test_string(env::current_dir().unwrap().display().to_string())),
        ("config", Config::default().into_value(Span::unknown())),
    ]
    .into_iter()
    .for_each(|(key, val)| engine_state.add_env_var(key.into(), val));

    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
}
