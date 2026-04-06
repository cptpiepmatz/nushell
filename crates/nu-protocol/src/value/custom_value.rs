use std::{cmp::Ordering, fmt, path::Path};

use crate::shell_error::generic::GenericError;
use crate::{ShellError, Span, Spanned, Type, Value, ast::Operator, casing::Casing};

/// Trait definition for a custom [`Value`](crate::Value) type
#[typetag::serde(tag = "type")]
pub trait CustomValue: fmt::Debug + Send + Sync {
    /// Custom `Clone` implementation
    ///
    /// This can reemit a `Value::Custom(Self, span)` or materialize another representation
    /// if necessary.
    fn clone_value(&self, span: Span) -> Value;

    //fn category(&self) -> Category;

    /// The friendly type name to show for the custom value, e.g. in `describe` and in error
    /// messages. This does not have to be the same as the name of the struct or enum, but
    /// conventionally often is.
    fn type_name(&self) -> String;

    /// Converts the custom value to a base nushell value.
    ///
    /// This imposes the requirement that you can represent the custom value in some form using the
    /// Value representations that already exist in nushell
    fn to_base_value(&self, span: Span) -> Result<Value, ShellError>;

    /// Any representation used to downcast object to its original type
    fn as_any(&self) -> &dyn std::any::Any;

    /// Any representation used to downcast object to its original type (mutable reference)
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;

    /// Follow cell path by numeric index (e.g. rows).
    ///
    /// Let `$val` be the custom value then these are the fields passed to this method:
    /// ```text
    ///      ╭── index [path_span]
    ///      ┴
    /// $val.0?
    /// ──┬─  ┬
    ///   │   ╰── optional, `true` if present
    ///   ╰── self [self_span]
    /// ```
    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
        optional: bool,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, index, optional);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    /// Follow cell path by string key (e.g. columns).
    ///
    /// Let `$val` be the custom value then these are the fields passed to this method:
    /// ```text
    ///         ╭── column_name [path_span]
    ///         │   ╭── casing, `Casing::Insensitive` if present
    ///      ───┴── ┴
    /// $val.column?!
    /// ──┬─       ┬
    ///   │        ╰── optional, `true` if present
    ///   ╰── self [self_span]
    /// ```
    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, column_name, optional, casing);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    /// ordering with other value (see [`std::cmp::PartialOrd`])
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }

    /// Definition of an operation between the object that implements the trait
    /// and another Value.
    ///
    /// The Operator enum is used to indicate the expected operation.
    ///
    /// Default impl raises [`ShellError::OperatorUnsupportedType`].
    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let _ = (lhs_span, right);
        Err(ShellError::OperatorUnsupportedType {
            op: operator,
            unsupported: Type::Custom(self.type_name().into()),
            op_span: op,
            unsupported_span: lhs_span,
            help: None,
        })
    }

    /// Save custom value to disk.
    ///
    /// This method is used in `save` to save a custom value to disk.
    /// This is done before opening any file, so saving can be handled differently.
    ///
    /// The default impl just returns an error.
    fn save(
        &self,
        path: Spanned<&Path>,
        value_span: Span,
        save_span: Span,
    ) -> Result<(), ShellError> {
        let _ = path;
        Err(ShellError::Generic(
            GenericError::new(
                "Cannot save custom value",
                format!("Saving custom value {} failed", self.type_name()),
                save_span,
            )
            .with_inner([ShellError::Generic(
                GenericError::new(
                    "Custom value does not implement `save`",
                    format!("{} doesn't implement saving to disk", self.type_name()),
                    value_span,
                )
                .with_help("Check the plugin's documentation for this value type. It might use a different way to save."),
            )]),
        ))
    }

    /// For custom values in plugins: return `true` here if you would like to be notified when all
    /// copies of this custom value are dropped in the engine.
    ///
    /// The notification will take place via `custom_value_dropped()` on the plugin type.
    ///
    /// The default is `false`.
    fn notify_plugin_on_drop(&self) -> bool {
        false
    }

    /// Returns an estimate of the memory size used by this CustomValue in bytes
    ///
    /// The default implementation returns the size of the trait object.
    fn memory_size(&self) -> usize {
        std::mem::size_of_val(self)
    }

    /// Returns `true` if this custom value should be iterable (like a list) when used with
    /// commands like `each`, `where`, etc.
    ///
    /// When this returns `true`, the engine will call `to_base_value()` to convert the custom
    /// value to a list before iteration. This is useful for lazy data structures like database
    /// query builders that should behave like lists when iterated.
    ///
    /// The default is `false`.
    fn is_iterable(&self) -> bool {
        false
    }

    /// Returns an iterator over the contents of this custom value.
    ///
    /// Commands such as `each`, `where`, and other list-like operations can use this to iterate
    /// over a custom value lazily instead of first converting it into a base value.
    ///
    /// Implement this method if the custom value can be iterated from front to back.
    ///
    /// The default implementation tries to reuse [`CustomValue::double_ended_iter`], since a
    /// double-ended iterator can also be used as a regular iterator.
    /// If only forward iteration is supported, override this method directly and leave
    /// [`CustomValue::double_ended_iter`] as is.
    ///
    /// Returning [`CustomValueCapability::Unsupported`] indicates that this custom value does not
    /// support lazy iteration.
    ///
    /// Returning [`CustomValueCapability::Error`] indicates that the value supports iteration, but
    /// creating the iterator failed.
    fn iter(&self) -> CustomValueCapability<Box<dyn CustomValueIterator>> {
        use CustomValueCapability as CVC;
        match self.double_ended_iter() {
            CVC::Unsupported => CVC::Unsupported,
            CVC::Error(err) => CVC::Error(err),
            CVC::Ready(iter) => CVC::Ready(iter as Box<dyn CustomValueIterator>),
        }
    }

    /// Returns a double-ended iterator over the contents of this custom value.
    ///
    /// A double-ended iterator supports both forward and backward iteration and can be used by
    /// commands that need to read values from the back or reverse the iteration order.
    ///
    /// Override this method if the custom value supports double-ended iteration. 
    /// Implementers only need to override this method, not [`CustomValue::iter`], because the 
    /// default implementation of [`CustomValue::iter`] automatically reuses a double-ended iterator
    /// as a regular iterator.
    ///
    /// The default implementation returns [`CustomValueCapability::Unsupported`].
    fn double_ended_iter(&self) -> CustomValueCapability<Box<dyn CustomValueDoubleEndedIterator>> {
        CustomValueCapability::Unsupported
    }
}

/// Result of requesting a capability from a [`CustomValue`].
///
/// This is used for optional features that a custom value may or may not implement,
/// such as lazy iteration.
///
/// A capability can be unsupported, successfully constructed, or fail during
/// construction.
pub enum CustomValueCapability<T> {
    /// The custom value does not implement this capability.
    Unsupported,

    /// The capability was successfully constructed and can be used.
    Ready(T),

    /// The custom value supports this capability, but constructing it failed.
    Error(ShellError),
}

/// Iterator trait for [`CustomValue`].
///
/// This is returned by [`CustomValue::iter`] and allows commands to consume a custom value lazily
/// instead of collecting it as another value first.
///
/// Iterators yield [`Value`] items directly. 
/// They may yield [`Value::Error`] to represent an error during iteration, or [`Value::Custom`] to
/// yield another custom value.
pub trait CustomValueIterator: Iterator<Item = Value> {
    /// Returns the exact number of remaining elements, if known.
    ///
    /// Commands that only need the remaining length of the iterator may use this instead of
    /// advancing the iterator.
    ///
    /// Unlike [`Iterator::size_hint`], this must be exact when present. 
    /// Return [`None`] if the remaining length is not known exactly.
    fn len(&self) -> Option<usize> {
        None
    }
}

/// Double-ended iterator trait for [`CustomValue`].
///
/// This extends [`CustomValueIterator`] with support for iterating from the back as well as the
/// front.
///
/// It can be returned from [`CustomValue::double_ended_iter`] to support commands that reverse the
/// iteration order or pull elements from the end.
pub trait CustomValueDoubleEndedIterator: CustomValueIterator + DoubleEndedIterator {}
