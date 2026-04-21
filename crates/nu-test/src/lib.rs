use nu_protocol::{Config, IntoValue, Span, engine::EngineState};

#[cfg(test)]
use nu_test_support::harness::main;

#[cfg(test)]
#[macro_use]
extern crate nu_test_support;

pub mod discover;
pub mod test;

pub fn engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let mut engine_state = nu_cmd_extra::add_extra_command_context(engine_state);

    engine_state.generate_nu_constant();
    [
        // ("PWD", Value::test_string(ROOT.to_string_lossy())),
        ("config", Config::default().into_value(Span::unknown())),
    ]
    .into_iter()
    .for_each(|(key, val)| engine_state.add_env_var(key.into(), val));

    nu_std::load_standard_library(&mut engine_state).expect("could not load standard library");

    engine_state
}
