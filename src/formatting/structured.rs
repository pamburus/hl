//! Formatting of records in machine-readable output formats, such as JSON and logfmt.
//!
//! Each record is written as a normalized record: the predefined fields first, in a fixed
//! order and under their output keys, then the remaining fields in source order, with every
//! key unique at its nesting level. Values keep their source types, and values from logfmt
//! sources are typed by the same rules as in the default output.

// std imports
use std::{cell::RefCell, marker::PhantomData, ops::Range, sync::Arc};

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    ExactIncludeExcludeKeyFilter, IncludeExcludeKeyFilter,
    filtering::{IncludeExcludeSetting, MatchOptions, NoNormalizing},
    model::{self, Caller, ObjectKey, RawValue, RecordFields},
    settings::PredefinedFields,
};

// relative imports
use super::RecordWithSourceFormatter;
use keyset::KeySet;

mod json;
mod keyset;
mod logfmt;

pub use json::JsonEncoder;

// ---

type Buf = Vec<u8>;

// ---

/// Where a number comes from, which tells whether its text is valid JSON number syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberSource {
    /// A JSON number, always in valid JSON syntax.
    Json,
    /// An unquoted logfmt value that hl types as a number, possibly not in valid JSON syntax.
    Logfmt,
}

// ---

/// Writes the parts of a normalized record in a specific output format.
///
/// An encoder holds only the small state it needs between calls, such as whether a separator
/// is due, so a copy of it together with the buffer length is a checkpoint to roll back to.
pub trait Encoder: Copy + Default {
    fn begin_record(&mut self, buf: &mut Buf);
    fn end_record(&mut self, buf: &mut Buf);
    /// Writes a key, followed by its value with one of the value methods.
    fn key(&mut self, buf: &mut Buf, key: &str);
    /// Starts an array item, followed by its value with one of the value methods.
    fn item(&mut self, buf: &mut Buf);
    fn string(&mut self, buf: &mut Buf, value: EncodedString);
    fn number(&mut self, buf: &mut Buf, value: &str, source: NumberSource);
    fn boolean(&mut self, buf: &mut Buf, value: bool);
    fn null(&mut self, buf: &mut Buf);
    fn begin_object(&mut self, buf: &mut Buf);
    fn end_object(&mut self, buf: &mut Buf);
    fn begin_array(&mut self, buf: &mut Buf);
    fn end_array(&mut self, buf: &mut Buf);

    /// Writes a string that is the concatenation of `parts`.
    fn text(&mut self, buf: &mut Buf, parts: &[&str]);

    /// Writes a string produced by `write`, which needs no escaping or quoting in any format.
    /// If `write` returns `false`, whatever it wrote is discarded and `false` is returned.
    #[inline]
    fn plain(&mut self, buf: &mut Buf, write: impl FnOnce(&mut Buf) -> bool) -> bool {
        let begin = buf.len();
        buf.push(b'"');
        if write(buf) {
            buf.push(b'"');
            true
        } else {
            buf.truncate(begin);
            false
        }
    }
}

// ---

/// Output keys of the predefined fields.
#[derive(Clone, Debug)]
struct OutputKeys {
    time: String,
    level: String,
    logger: String,
    message: String,
    caller: String,
}

impl Default for OutputKeys {
    fn default() -> Self {
        Self {
            time: "timestamp".into(),
            level: "level".into(),
            logger: "logger".into(),
            message: "message".into(),
            caller: "caller".into(),
        }
    }
}

// ---

#[derive(Default, Clone)]
pub struct StructuredRecordFormatterBuilder {
    flatten: bool,
    hide_empty_fields: bool,
    fields: Option<Arc<IncludeExcludeKeyFilter>>,
    predefined_fields: Option<Arc<ExactIncludeExcludeKeyFilter>>,
}

impl StructuredRecordFormatterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether nested objects are flattened into dotted keys.
    pub fn with_flatten(self, flatten: bool) -> Self {
        Self { flatten, ..self }
    }

    /// Sets whether empty fields, including empty predefined fields, are left out.
    pub fn with_empty_fields_hiding(self, enabled: bool) -> Self {
        Self {
            hide_empty_fields: enabled,
            ..self
        }
    }

    /// Sets the user's field visibility filter, which also applies to predefined fields by their output keys.
    pub fn with_field_filter(self, value: Arc<IncludeExcludeKeyFilter>) -> Self {
        Self {
            fields: Some(value),
            ..self
        }
    }

    /// Sets the predefined fields, whose nested source fields are left out of the regular fields.
    pub fn with_predefined_fields(self, predefined: &PredefinedFields) -> Self {
        let mut filter = ExactIncludeExcludeKeyFilter::new(MatchOptions::<NoNormalizing>::default());
        for name in predefined.nested_field_names() {
            filter.entry(name).exclude();
        }
        Self {
            predefined_fields: Some(Arc::new(filter)),
            ..self
        }
    }

    pub fn build<E: Encoder>(self) -> StructuredRecordFormatter<E> {
        let keys = OutputKeys::default();
        let fields = self.fields.unwrap_or_default();
        let visible = |key: &str| Context::root(Some(&*fields), None).child(key).is_some();
        let visible = Visibility {
            time: visible(&keys.time),
            level: visible(&keys.level),
            logger: visible(&keys.logger),
            message: visible(&keys.message),
            caller: visible(&keys.caller),
        };

        StructuredRecordFormatter {
            keys,
            visible,
            flatten: self.flatten,
            hide_empty_fields: self.hide_empty_fields,
            fields,
            predefined_fields: self.predefined_fields.unwrap_or_default(),
            encoder: PhantomData,
        }
    }
}

// ---

/// Visibility of the predefined fields under the user's field filter.
#[derive(Clone, Copy, Debug)]
struct Visibility {
    time: bool,
    level: bool,
    logger: bool,
    message: bool,
    caller: bool,
}

// ---

/// Formats records as normalized records with an [`Encoder`].
pub struct StructuredRecordFormatter<E> {
    keys: OutputKeys,
    visible: Visibility,
    flatten: bool,
    hide_empty_fields: bool,
    fields: Arc<IncludeExcludeKeyFilter>,
    predefined_fields: Arc<ExactIncludeExcludeKeyFilter>,
    encoder: PhantomData<fn() -> E>,
}

impl<E: Encoder> StructuredRecordFormatter<E> {
    /// Formats a record.
    ///
    /// Working memory comes from a per-thread scratch that is reused for every record,
    /// so formatting allocates nothing once the scratch has grown to fit the records.
    pub fn format_record(&self, buf: &mut Buf, rec: &model::Record) {
        SCRATCH.with(|scratch| match scratch.try_borrow_mut() {
            Ok(mut scratch) => self.format_record_with(buf, rec, &mut scratch),
            Err(_) => self.format_record_with(buf, rec, &mut Scratch::default()),
        })
    }

    fn format_record_with(&self, buf: &mut Buf, rec: &model::Record, s: &mut Scratch) {
        let mut w = Writer {
            buf,
            e: E::default(),
            emitted: KeySet::default(),
        };

        w.e.begin_record(w.buf);
        self.format_predefined(&mut w, s, rec);
        let ctx = Context::root(Some(&*self.fields), Some(&*self.predefined_fields));
        self.format_fields(&mut w, s, Fields::Record(&rec.fields), ctx, true);
        w.e.end_record(w.buf);
    }

    fn format_predefined<'s>(&'s self, w: &mut Writer<'_, 's, E>, s: &mut Scratch, rec: &model::Record) {
        if let (Some(ts), true) = (&rec.ts, self.visible.time) {
            w.key_predefined(&self.keys.time);
            if !w.e.plain(w.buf, |buf| ts.format_rfc3339_utc(buf)) {
                match ts.source() {
                    Some(value) => self.format_value(w, s, value),
                    None => w.e.string(w.buf, EncodedString::raw(ts.raw())),
                }
            }
        }

        if let (Some(level), true) = (rec.level, self.visible.level) {
            w.key_predefined(&self.keys.level);
            w.e.text(w.buf, &[level.as_ref()]);
        }

        if let (Some(logger), true) = (rec.logger, self.visible.logger) {
            w.key_predefined(&self.keys.logger);
            w.e.text(w.buf, &[logger]);
        }

        if let (Some(message), true) = (rec.message, self.visible.message) {
            if !self.hide_empty_fields || !message.is_empty() {
                w.key_predefined(&self.keys.message);
                self.format_value(w, s, message);
            }
        }

        if !rec.caller.is_empty() && self.visible.caller {
            w.key_predefined(&self.keys.caller);
            format_caller(&mut w.e, w.buf, &rec.caller);
        }
    }

    /// Formats the fields of one nesting level, applying filters, resolving repeated keys
    /// and dropping collisions with predefined output keys.
    ///
    /// Returns `true` if any field was written.
    fn format_fields(&self, w: &mut Writer<E>, s: &mut Scratch, fields: Fields, ctx: Context, top: bool) -> bool {
        let level = s.resolve(fields, self.flatten);

        if level.dotted {
            // Keys with dots at this level can become equal to flattened keys of nested
            // objects, so all keys of this subtree are resolved together.
            s.end(level);
            return self.format_leaves(w, s, fields, ctx, top);
        }

        let mut any = false;
        for i in 0..level.len {
            let (first, last) = s.entry(&level, i);
            any |= self.format_field(w, s, fields.key(first), fields.value(last), ctx, top);
        }
        s.end(level);
        any
    }

    /// Formats a field. Returns `true` if anything was written.
    fn format_field(
        &self,
        w: &mut Writer<E>,
        s: &mut Scratch,
        key: &str,
        value: RawValue,
        ctx: Context,
        top: bool,
    ) -> bool {
        if self.hide_empty_fields && value.is_empty() {
            return false;
        }
        let Some(child) = ctx.child(key) else {
            return false;
        };

        let len = s.path.len();
        if self.flatten {
            s.push_path(key);
        }
        let collision = top && w.emitted.contains(if self.flatten { &s.path } else { key });

        let written = match value {
            _ if collision => false,
            RawValue::Object(object) => {
                let object = object.parse().unwrap_or_default();
                if object.fields.is_empty() {
                    w.key(s, key, self.flatten);
                    w.e.begin_object(w.buf);
                    w.e.end_object(w.buf);
                    true
                } else if self.flatten {
                    self.format_fields(w, s, Fields::Object(&object.fields), child, top)
                } else {
                    let checkpoint = (w.buf.len(), w.e);
                    w.key(s, key, false);
                    w.e.begin_object(w.buf);
                    let any = self.format_fields(w, s, Fields::Object(&object.fields), child, false);
                    if !any && (self.hide_empty_fields || child.has_predefined_filter()) {
                        w.buf.truncate(checkpoint.0);
                        w.e = checkpoint.1;
                        false
                    } else {
                        w.e.end_object(w.buf);
                        true
                    }
                }
            }
            _ => {
                w.key(s, key, self.flatten);
                self.format_value(w, s, value);
                true
            }
        };

        s.path.truncate(len);
        written
    }

    /// Formats a subtree whose flattened keys are resolved together, as some of its keys have dots.
    ///
    /// The subtree is walked twice: first to find the flattened keys of its leaves and where
    /// each key occurs first and last, then to write each key at its first occurrence.
    /// When the value comes from a later occurrence, the subtree is walked again to find it.
    fn format_leaves(&self, w: &mut Writer<E>, s: &mut Scratch, fields: Fields, ctx: Context, top: bool) -> bool {
        let base = s.leaves.len();
        let text_base = s.leaf_keys.len();

        self.walk_leaves(s, fields, ctx, &mut |s, _| {
            let begin = s.leaf_keys.len();
            s.leaf_keys.push_str(&s.path);
            let key = begin..s.leaf_keys.len();
            let hash = hash(&s.path);
            let i = s.leaves.len();
            let first = s.leaves[base..].iter().position(|leaf| {
                leaf.last.is_some() && leaf.hash == hash && s.leaf_keys[leaf.key.clone()] == s.leaf_keys[key.clone()]
            });
            let last = match first {
                Some(first) => {
                    s.leaves[base + first].last = Some(i);
                    None
                }
                None => Some(i),
            };
            s.leaves.push(Leaf { hash, key, last });
        });

        let mut any = false;
        let mut i = base;
        self.walk_leaves(s, fields, ctx, &mut |s, value| {
            let leaf = i;
            i += 1;
            let Some(last) = s.leaves[leaf].last else {
                return;
            };
            if top && w.emitted.contains(&s.path) {
                return;
            }
            let value = if last == leaf {
                value
            } else {
                let mut value = value;
                let mut j = base;
                self.walk_leaves(s, fields, ctx, &mut |_, v| {
                    if j == last {
                        value = v;
                    }
                    j += 1;
                });
                value
            };
            w.key(s, "", true);
            self.format_value(w, s, value);
            any = true;
        });

        s.leaves.truncate(base);
        s.leaf_keys.truncate(text_base);
        any
    }

    /// Calls `visit` for each visible leaf of a subtree, in order, with the flattened key
    /// of the leaf in the scratch path. Objects are flattened, and arrays and empty objects are leaves.
    fn walk_leaves<'a>(
        &self,
        s: &mut Scratch,
        fields: Fields<'_, 'a>,
        ctx: Context,
        visit: &mut dyn FnMut(&mut Scratch, RawValue<'a>),
    ) {
        let level = s.resolve(fields, false);
        for i in 0..level.len {
            let (first, last) = s.entry(&level, i);
            let (key, value) = (fields.key(first), fields.value(last));
            if self.hide_empty_fields && value.is_empty() {
                continue;
            }
            let Some(child) = ctx.child(key) else {
                continue;
            };
            let len = s.path.len();
            s.push_path(key);
            match value {
                RawValue::Object(object) => {
                    let object = object.parse().unwrap_or_default();
                    if object.fields.is_empty() {
                        visit(s, value);
                    } else {
                        self.walk_leaves(s, Fields::Object(&object.fields), child, visit);
                    }
                }
                _ => visit(s, value),
            }
            s.path.truncate(len);
        }
        s.end(level);
    }

    /// Formats a value with no filters, as in arrays and predefined fields.
    fn format_value(&self, w: &mut Writer<E>, s: &mut Scratch, value: RawValue) {
        match value {
            RawValue::String(EncodedString::Raw(raw)) => match RawValue::auto(raw.as_str()) {
                RawValue::Number(value) => w.e.number(w.buf, value, NumberSource::Logfmt),
                RawValue::String(_) => w.e.string(w.buf, EncodedString::Raw(raw)),
                value => self.format_value(w, s, value),
            },
            RawValue::String(value) => w.e.string(w.buf, value),
            RawValue::Number(value) => w.e.number(w.buf, value, NumberSource::Json),
            RawValue::Boolean(value) => w.e.boolean(w.buf, value),
            RawValue::Null => w.e.null(w.buf),
            RawValue::Object(object) => {
                let object = object.parse().unwrap_or_default();
                let fields = Fields::Object(&object.fields);
                w.e.begin_object(w.buf);
                let level = s.resolve(fields, false);
                for i in 0..level.len {
                    let (first, last) = s.entry(&level, i);
                    w.e.key(w.buf, fields.key(first));
                    self.format_value(w, s, fields.value(last));
                }
                s.end(level);
                w.e.end_object(w.buf);
            }
            RawValue::Array(array) => {
                let array = array.parse::<32>().unwrap_or_default();
                w.e.begin_array(w.buf);
                for value in array.iter() {
                    w.e.item(w.buf);
                    self.format_value(w, s, *value);
                }
                w.e.end_array(w.buf);
            }
        }
    }
}

impl<E: Encoder> RecordWithSourceFormatter for StructuredRecordFormatter<E> {
    /// Formats a record. The prefix is not written, as structured output holds nothing but records.
    #[inline]
    fn format_record(&self, buf: &mut Buf, _prefix: Range<usize>, rec: model::RecordWithSource) {
        StructuredRecordFormatter::format_record(self, buf, rec.record)
    }
}

// ---

/// The output of a record being formatted.
struct Writer<'b, 's, E> {
    buf: &'b mut Buf,
    e: E,
    /// Output keys of the predefined fields written so far.
    emitted: KeySet<'s>,
}

impl<'s, E: Encoder> Writer<'_, 's, E> {
    /// Writes the output key of a predefined field.
    #[inline]
    fn key_predefined(&mut self, key: &'s str) {
        self.emitted.insert(key);
        self.e.key(self.buf, key);
    }

    /// Writes `key`, or the flattened key in the scratch path when flattening.
    #[inline]
    fn key(&mut self, s: &Scratch, key: &str, flatten: bool) {
        self.e.key(self.buf, if flatten { &s.path } else { key });
    }
}

// ---

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// Working memory for formatting records, reused from record to record.
///
/// Nesting levels use the stacks in it in turn, each truncating them back when it is done.
#[derive(Default)]
struct Scratch {
    /// The flattened key of the current field.
    path: String,
    /// Key hashes and field indices of the nesting levels being resolved.
    keys: Vec<(u64, usize)>,
    /// Field indices of the first and the last occurrence of each key, for resolved nesting levels with repeated keys.
    order: Vec<(usize, usize)>,
    /// Leaves of a subtree whose flattened keys are resolved together.
    leaves: Vec<Leaf>,
    /// Flattened keys of the leaves.
    leaf_keys: String,
}

impl Scratch {
    #[inline]
    fn push_path(&mut self, key: &str) {
        if !self.path.is_empty() {
            self.path.push('.');
        }
        self.path.push_str(key);
    }

    /// Resolves the repeated keys of a nesting level, which must be ended with [`Scratch::end`].
    ///
    /// Each key is kept at the position of its first occurrence, with the value of its last occurrence.
    #[inline]
    fn resolve(&mut self, fields: Fields, flatten: bool) -> ResolvedLevel {
        let base = self.keys.len();
        let mut dotted = false;
        let mut repeats = false;
        for i in 0..fields.len() {
            let key = fields.key(i);
            dotted |= flatten && key.contains('.');
            let hash = hash(key);
            if self.keys[base..]
                .iter()
                .any(|&(h, j)| h == hash && fields.key(j) == key)
            {
                repeats = true;
            } else {
                self.keys.push((hash, i));
            }
        }

        let order = self.order.len();
        let len = if repeats {
            for &(_, first) in &self.keys[base..] {
                let key = fields.key(first);
                let last = (first..fields.len())
                    .rev()
                    .find(|&j| fields.key(j) == key)
                    .unwrap_or(first);
                self.order.push((first, last));
            }
            self.order.len() - order
        } else {
            fields.len()
        };
        self.keys.truncate(base);

        ResolvedLevel {
            len,
            order: repeats.then_some(order),
            dotted,
        }
    }

    /// Returns the field indices of the first and the last occurrence of the `i`-th key of a resolved level.
    #[inline]
    fn entry(&self, level: &ResolvedLevel, i: usize) -> (usize, usize) {
        match level.order {
            Some(order) => self.order[order + i],
            None => (i, i),
        }
    }

    /// Ends a resolved level, releasing its part of the scratch.
    #[inline]
    fn end(&mut self, level: ResolvedLevel) {
        if let Some(order) = level.order {
            self.order.truncate(order);
        }
    }
}

/// A nesting level with resolved repeated keys.
struct ResolvedLevel {
    /// Number of distinct keys.
    len: usize,
    /// Start of the level's entries in the scratch order, if some keys are repeated.
    order: Option<usize>,
    /// Whether some keys have dots while flattening, which may make them equal to flattened keys.
    dotted: bool,
}

/// A leaf of a subtree whose flattened keys are resolved together.
struct Leaf {
    hash: u64,
    /// Range of the flattened key in the scratch leaf keys.
    key: Range<usize>,
    /// For the first leaf with a key, the index of the last leaf with the same key.
    last: Option<usize>,
}

// ---

/// Fields of a nesting level, accessed by index.
#[derive(Clone, Copy)]
enum Fields<'k, 'a> {
    Record(&'k RecordFields<'a>),
    Object(&'k ObjectFields<'a>),
}

type ObjectFields<'a> = heapopt::Vec<(ObjectKey<'a>, RawValue<'a>), 32>;

impl<'k, 'a> Fields<'k, 'a> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            Self::Record(fields) => fields.len(),
            Self::Object(fields) => fields.len(),
        }
    }

    #[inline]
    fn key(&self, i: usize) -> &'k str {
        match self {
            Self::Record(fields) => fields[i].0,
            Self::Object(fields) => &fields[i].0,
        }
    }

    #[inline]
    fn value(&self, i: usize) -> RawValue<'a> {
        match self {
            Self::Record(fields) => fields[i].1,
            Self::Object(fields) => fields[i].1,
        }
    }
}

#[inline]
fn hash(key: &str) -> u64 {
    wyhash::wyhash(key.as_bytes(), 0)
}

// ---

/// The field filters that apply at a nesting level.
#[derive(Clone, Copy)]
struct Context<'f> {
    filter: Option<&'f IncludeExcludeKeyFilter>,
    setting: IncludeExcludeSetting,
    predefined: Option<&'f ExactIncludeExcludeKeyFilter>,
    predefined_setting: IncludeExcludeSetting,
}

impl<'f> Context<'f> {
    fn root(filter: Option<&'f IncludeExcludeKeyFilter>, predefined: Option<&'f ExactIncludeExcludeKeyFilter>) -> Self {
        Self {
            filter,
            setting: IncludeExcludeSetting::Unspecified,
            predefined,
            predefined_setting: IncludeExcludeSetting::Unspecified,
        }
    }

    /// Returns the context for the field with `key`, or `None` if the field is hidden,
    /// by the user's filter or as a nested source field of a predefined field.
    #[inline]
    fn child(&self, key: &str) -> Option<Self> {
        let (predefined, predefined_setting, leaf) = descend(self.predefined, self.predefined_setting, key);
        if predefined_setting == IncludeExcludeSetting::Exclude && leaf {
            return None;
        }
        let (filter, setting, leaf) = descend(self.filter, self.setting, key);
        if setting == IncludeExcludeSetting::Exclude && leaf {
            return None;
        }
        Some(Self {
            filter,
            setting,
            predefined,
            predefined_setting,
        })
    }

    /// Tells whether some nested source fields of predefined fields are below this level.
    #[inline]
    fn has_predefined_filter(&self) -> bool {
        self.predefined.is_some()
    }
}

#[inline]
fn descend<'f, N: crate::filtering::KeyNormalize>(
    filter: Option<&'f crate::filtering::IncludeExcludeKeyFilter<N>>,
    setting: IncludeExcludeSetting,
    key: &str,
) -> (
    Option<&'f crate::filtering::IncludeExcludeKeyFilter<N>>,
    IncludeExcludeSetting,
    bool,
) {
    match filter {
        Some(filter) => {
            let setting = setting.apply(filter.setting());
            match filter.get(key) {
                Some(child) => (Some(child), setting.apply(child.setting()), child.leaf()),
                None => (None, setting, true),
            }
        }
        None => (None, setting, true),
    }
}

// ---

/// Formats the caller as one string: `name`, `file:line`, `file` or `name @ file:line`.
#[inline]
fn format_caller<E: Encoder>(e: &mut E, buf: &mut Buf, caller: &Caller) {
    let location = !caller.file.is_empty() || !caller.line.is_empty();
    let line = if caller.line.is_empty() {
        ["", ""]
    } else {
        [":", caller.line]
    };
    match (caller.name.is_empty(), location) {
        (false, false) => e.text(buf, &[caller.name]),
        (true, _) => e.text(buf, &[caller.file, line[0], line[1]]),
        (false, true) => e.text(buf, &[caller.name, " @ ", caller.file, line[0], line[1]]),
    }
}

#[cfg(test)]
mod proptests;
#[cfg(test)]
mod tests;
