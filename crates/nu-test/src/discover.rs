use nu_engine::scope::ScopeData;
use nu_protocol::{
    Id, Span,
    engine::{DEFAULT_OVERLAY_NAME, Stack, StateWorkingSet},
};

#[test]
fn discover() {
    let mut engine_state = super::engine_state();
    let stack = Stack::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let code = b"overlay new testing; use tests/example.nu";
    let block = nu_parser::parse(&mut working_set, None, code, false);
    assert!(working_set.parse_errors.is_empty());
    assert!(working_set.compile_errors.is_empty());
    engine_state.merge_delta(working_set.delta).unwrap();

    dbg!(engine_state.scope.overlays.iter().map(|(name, _)| str::from_utf8(name).unwrap()).collect::<Vec<_>>());

    let command_attributes: Vec<_> = engine_state
        .scope
        .active_overlays(&mut vec![DEFAULT_OVERLAY_NAME.as_bytes().to_vec()])
        .map(|overlay| overlay.decls.values())
        .flatten()
        .map(|decl_id| engine_state.get_decl(*decl_id))
        .map(|command| (command.name(), command.attributes(), command.decl_span()))
        .collect();

    dbg!(command_attributes);

    todo!()
}
