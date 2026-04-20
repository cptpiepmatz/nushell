use std::any::Any;

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
    let code = b"overlay new testing; source tests/example.nu";
    let block = nu_parser::parse(&mut working_set, None, code, false);
    assert!(working_set.parse_errors.is_empty());
    assert!(working_set.compile_errors.is_empty());
    engine_state.merge_delta(working_set.delta).unwrap();

    let command_attributes: Vec<_> = engine_state
        .scope
        .active_overlays(&mut vec![DEFAULT_OVERLAY_NAME.as_bytes().to_vec()])
        .map(|overlay| overlay.decls.values())
        .flatten()
        .map(|decl_id| engine_state.get_decl(*decl_id))
        .inspect(|command| {dbg!(command.name());})
        .flat_map(|command| command.block_id().map(|block_id| (command, block_id)))
        .map(|(command, block_id)| (command.name(), command.attributes(), engine_state.get_block(block_id).span))
        .map(|(name, attributes, span)| engine_state.try_get_file_contents(span.unwrap()).map(str::from_utf8))
        .collect();

    dbg!(command_attributes);

    todo!()
}
