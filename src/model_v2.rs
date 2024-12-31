// std imports
use std::collections::HashMap;

// third-party imports
use titlecase::titlecase;
use wildflower::Pattern;

// workspace imports
use encstr::EncodedString;

// local imports
use crate::{
    app::UnixTimestampUnit,
    ast,
    model::{Caller, Level},
    settings::PredefinedFields,
    timestamp::Timestamp,
};

// ---

const MAX_PREDEFINED_FIELDS: usize = 8;

// ---

pub struct Record<'s> {
    pub ts: Option<Timestamp<'s>>,
    pub message: Option<Value<'s>>,
    pub level: Option<Level>,
    pub logger: Option<&'s str>,
    pub caller: Option<Caller<'s>>,
    pub fields: RecordFields<'s>,
    predefined: heapless::Vec<Field<'s>, MAX_PREDEFINED_FIELDS>,
}

impl<'s> Record<'s> {
    #[inline]
    pub fn new(fields: RecordFields<'s>) -> Self {
        Self {
            ts: None,
            message: None,
            level: None,
            logger: None,
            caller: None,
            fields,
            predefined: heapless::Vec::new(),
        }
    }

    #[inline]
    pub fn fields_for_search<'r>(&'r self) -> impl Iterator<Item = Field<'s>> + 'r {
        self.fields.iter().chain(self.predefined.iter().copied())
    }

    #[inline]
    pub fn matches<F: RecordFilter>(&self, filter: F) -> bool {
        filter.apply(self)
    }
}

// ---

pub struct RecordFields<'s> {
    inner: ast::Children<'s>,
}

impl<'s> RecordFields<'s> {
    #[inline]
    fn new(inner: ast::Children<'s>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn iter(&self) -> RecordFieldsIter<'s> {
        RecordFieldsIter::new(self.inner.iter())
    }
}

// ---

struct RecordFieldsIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> RecordFieldsIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for RecordFieldsIter<'s> {
    type Item = Field<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub enum Value<'s> {
    Null,
    Bool(bool),
    Number(&'s str),
    String(EncodedString<'s>),
    Array(Array<'s>),
    Object(Object<'s>),
}

impl<'s> From<ast::Node<'s>> for Value<'s> {
    fn from(node: ast::Node<'s>) -> Self {
        match *node.value() {
            ast::Value::Scalar(scalar) => match scalar {
                ast::Scalar::Null => Self::Null,
                ast::Scalar::Bool(b) => Self::Bool(b),
                ast::Scalar::Number(s) => Self::Number(s),
                ast::Scalar::String(s) => Self::String(s.into()),
            },
            ast::Value::Array => Self::Array(Array::new(node)),
            ast::Value::Object => Self::Object(Object::new(node)),
            _ => panic!("expected scalar, array or object node, got {:?}", node),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Array<'s> {
    inner: ast::Node<'s>,
}

impl<'s> Array<'s> {
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> From<Array<'s>> for Value<'s> {
    #[inline]
    fn from(a: Array<'s>) -> Self {
        Value::Array(a)
    }
}

impl<'s> IntoIterator for Array<'s> {
    type Item = Value<'s>;
    type IntoIter = ArrayIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self.inner.children().iter())
    }
}

// ---

struct ArrayIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> ArrayIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for ArrayIter<'s> {
    type Item = Value<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Value::from)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Object<'s> {
    inner: ast::Node<'s>,
}

impl<'s> Object<'s> {
    fn new(inner: ast::Node<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> From<Object<'s>> for Value<'s> {
    #[inline]
    fn from(o: Object<'s>) -> Self {
        Value::Object(o)
    }
}

impl<'s> IntoIterator for Object<'s> {
    type Item = Field<'s>;
    type IntoIter = ObjectIter<'s>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ObjectIter::new(self.inner.children().iter())
    }
}

// ---

struct ObjectIter<'s> {
    inner: ast::SiblingsIter<'s>,
}

impl<'s> ObjectIter<'s> {
    #[inline]
    fn new(inner: ast::SiblingsIter<'s>) -> Self {
        Self { inner }
    }
}

impl<'s> Iterator for ObjectIter<'s> {
    type Item = Field<'s>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Field::from_node)
    }
}

// ---

#[derive(Debug, Clone, Copy)]
pub struct Field<'s> {
    pub key: &'s str,
    pub value: Value<'s>,
}

impl<'s> Field<'s> {
    #[inline]
    pub fn new(key: &'s str, value: Value<'s>) -> Self {
        Self { key, value }
    }

    #[inline]
    fn from_node(node: ast::Node<'s>) -> Self {
        let ast::Value::Key(key) = node.value() else {
            panic!("expected key node, got {:?}", node.value());
        };

        Field {
            key: key.source(),
            value: node.children().iter().next().unwrap().into(),
        }
    }
}

// ---

pub trait RecordWithSourceConstructor<'r, 's> {
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's>;
}
// ---

pub struct RecordWithSource<'r, 's> {
    pub record: &'r Record<'s>,
    pub source: &'s [u8],
}

impl<'r, 's> RecordWithSource<'r, 's> {
    #[inline]
    pub fn new(record: &'r Record<'s>, source: &'s [u8]) -> Self {
        Self { record, source }
    }
}

impl<'r, 's> RecordWithSourceConstructor<'r, 's> for Record<'s> {
    #[inline]
    fn with_source(&'r self, source: &'s [u8]) -> RecordWithSource<'r, 's> {
        RecordWithSource::new(self, source)
    }
}

// ---

pub trait RecordFilter {
    fn apply<'a>(&self, record: &Record<'a>) -> bool;

    #[inline]
    fn and<F>(self, rhs: F) -> RecordFilterAnd<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterAnd { lhs: self, rhs }
    }

    #[inline]
    fn or<F>(self, rhs: F) -> RecordFilterOr<Self, F>
    where
        Self: Sized,
        F: RecordFilter,
    {
        RecordFilterOr { lhs: self, rhs }
    }
}

impl<T: RecordFilter + ?Sized> RecordFilter for Box<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl<T: RecordFilter> RecordFilter for &T {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        (**self).apply(record)
    }
}

impl RecordFilter for Level {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        record.level.map_or(false, |x| x <= *self)
    }
}

impl<T: RecordFilter> RecordFilter for Option<T> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        if let Some(filter) = self {
            filter.apply(record)
        } else {
            true
        }
    }
}

// ---

pub struct RecordFilterAnd<L: RecordFilter, R: RecordFilter> {
    lhs: L,
    rhs: R,
}

impl<L: RecordFilter, R: RecordFilter> RecordFilter for RecordFilterAnd<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) && self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterOr<L: RecordFilter, R: RecordFilter> {
    lhs: L,
    rhs: R,
}

impl<L: RecordFilter, R: RecordFilter> RecordFilter for RecordFilterOr<L, R> {
    #[inline]
    fn apply<'a>(&self, record: &Record<'a>) -> bool {
        self.lhs.apply(record) || self.rhs.apply(record)
    }
}

// ---

pub struct RecordFilterNone;

impl RecordFilter for RecordFilterNone {
    #[inline]
    fn apply<'a>(&self, _: &Record<'a>) -> bool {
        true
    }
}

// ---

pub struct ParserSettings {
    unix_ts_unit: Option<UnixTimestampUnit>,
    level: Vec<(HashMap<String, Level>, Option<Level>)>,
    blocks: Vec<ParserSettingsBlock>,
    ignore: Vec<Pattern<String>>,
}

impl ParserSettings {
    pub fn new<'a, I: IntoIterator<Item = &'a String>>(
        predefined: &PredefinedFields,
        ignore: I,
        unix_ts_unit: Option<UnixTimestampUnit>,
    ) -> Self {
        let mut result = Self {
            unix_ts_unit,
            level: Vec::new(),
            blocks: vec![ParserSettingsBlock::default()],
            ignore: ignore.into_iter().map(|x| Pattern::new(x.to_string())).collect(),
        };

        result.init(predefined);
        result
    }

    fn init(&mut self, pf: &PredefinedFields) {
        self.build_block(0, &pf.time.names, FieldSettings::Time, 0);
        self.build_block(0, &pf.message.names, FieldSettings::Message, 0);
        self.build_block(0, &pf.logger.names, FieldSettings::Logger, 0);
        self.build_block(0, &pf.caller.names, FieldSettings::Caller, 0);
        self.build_block(0, &pf.caller_file.names, FieldSettings::CallerFile, 0);
        self.build_block(0, &pf.caller_line.names, FieldSettings::CallerLine, 0);

        let mut j = 0;
        for variant in &pf.level.variants {
            let mut mapping = HashMap::new();
            for (level, values) in &variant.values {
                for value in values {
                    mapping.insert(value.clone(), level.clone());
                    mapping.insert(value.to_lowercase(), level.clone());
                    mapping.insert(value.to_uppercase(), level.clone());
                    mapping.insert(titlecase(value), level.clone());
                }
            }
            let k = self.level.len();
            self.level.push((mapping.clone(), variant.level));
            self.build_block(0, &variant.names, FieldSettings::Level(k), j);
            j += variant.names.len();
        }
    }

    fn build_block<'a, N: IntoIterator<Item = &'a String>>(
        &mut self,
        n: usize,
        names: N,
        settings: FieldSettings,
        priority: usize,
    ) {
        for (i, name) in names.into_iter().enumerate() {
            self.build_block_for_name(n, name, settings, priority + i)
        }
    }

    fn build_block_for_name(&mut self, n: usize, name: &String, settings: FieldSettings, priority: usize) {
        self.blocks[n].fields.insert(name.clone(), (settings, priority));
        let mut remainder = &name[..];
        while let Some(k) = remainder.rfind('.') {
            let (name, nested) = name.split_at(k);
            let nested = &nested[1..];

            let nest = self.blocks[n]
                .fields
                .get(name)
                .and_then(|f| {
                    if let FieldSettings::Nested(nest) = f.0 {
                        Some(nest)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let nest = self.blocks.len();
                    self.blocks.push(ParserSettingsBlock::default());
                    self.blocks[n]
                        .fields
                        .insert(name.to_string(), (FieldSettings::Nested(nest), priority));
                    nest
                });

            self.build_block_for_name(nest, &nested.into(), settings, priority);
            remainder = name;
        }
    }

    #[inline]
    fn apply<'a>(&self, key: &'a str, value: Value<'a>, to: &mut Record<'a>, pc: &mut PriorityController) {
        self.blocks[0].apply(self, key, value, to, pc, true);
    }

    #[inline]
    fn apply_each<'a, 'i, I>(&self, items: I, to: &mut Record<'a>)
    where
        I: IntoIterator<Item = &'i (&'a str, Value<'a>)>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    #[inline]
    fn apply_each_ctx<'a, 'i, I>(&self, items: I, to: &mut Record<'a>, pc: &mut PriorityController)
    where
        I: IntoIterator<Item = &'i (&'a str, Value<'a>)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(key, *value, to, pc)
        }
    }
}

impl Default for ParserSettings {
    #[inline]
    fn default() -> Self {
        Self::new(&PredefinedFields::default(), Vec::new(), None)
    }
}

// ---

#[derive(Default, Debug)]
struct ParserSettingsBlock {
    fields: HashMap<String, (FieldSettings, usize)>,
}

impl ParserSettingsBlock {
    fn apply<'a>(
        &self,
        ps: &ParserSettings,
        key: &'a str,
        value: Value<'a>,
        to: &mut Record<'a>,
        pc: &mut PriorityController,
        is_root: bool,
    ) {
        let done = match self.fields.get(key) {
            Some((field, priority)) => {
                let kind = field.kind();
                if let Some(kind) = kind {
                    pc.prioritize(kind, *priority, |pc| field.apply_ctx(ps, value, to, pc))
                } else {
                    field.apply_ctx(ps, value, to, pc);
                    false
                }
            }
            None => false,
        };
        if is_root && done {
            to.predefined.push(Field::new(key, value)).ok();
        }
        if done || !is_root {
            return;
        }

        for pattern in &ps.ignore {
            if pattern.matches(key) {
                return;
            }
        }
        to.fields.push(Field::new(key, value));
    }

    #[inline]
    fn apply_each_ctx<'a, 'i, I>(
        &self,
        ps: &ParserSettings,
        items: I,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
        is_root: bool,
    ) where
        I: IntoIterator<Item = &'i (&'a str, RawValue<'a>)>,
        'a: 'i,
    {
        for (key, value) in items {
            self.apply(ps, key, *value, to, ctx, is_root)
        }
    }
}

// ---

#[derive(Default)]
struct PriorityController {
    time: Option<usize>,
    level: Option<usize>,
    logger: Option<usize>,
    message: Option<usize>,
    caller: Option<usize>,
    caller_file: Option<usize>,
    caller_line: Option<usize>,
}

impl PriorityController {
    #[inline]
    fn prioritize<F: FnOnce(&mut Self) -> bool>(&mut self, kind: FieldKind, priority: usize, update: F) -> bool {
        let p = match kind {
            FieldKind::Time => &mut self.time,
            FieldKind::Level => &mut self.level,
            FieldKind::Logger => &mut self.logger,
            FieldKind::Message => &mut self.message,
            FieldKind::Caller => &mut self.caller,
            FieldKind::CallerFile => &mut self.caller_file,
            FieldKind::CallerLine => &mut self.caller_line,
        };

        if p.is_none() || Some(priority) <= *p {
            *p = Some(priority);
            update(self)
        } else {
            false
        }
    }
}

// ---

#[derive(Clone, Copy, Debug)]
enum FieldSettings {
    Time,
    Level(usize),
    Logger,
    Message,
    Caller,
    CallerFile,
    CallerLine,
    Nested(usize),
}

impl FieldSettings {
    fn apply<'a>(&self, ps: &ParserSettings, value: RawValue<'a>, to: &mut Record<'a>) -> bool {
        match *self {
            Self::Time => {
                let s = value.raw_str();
                let s = if s.as_bytes()[0] == b'"' { &s[1..s.len() - 1] } else { s };
                let ts = Timestamp::new(s).with_unix_unit(ps.unix_ts_unit);
                to.ts = Some(ts);
                true
            }
            Self::Level(i) => {
                let value = value.parse().ok().unwrap_or_else(|| value.raw_str());
                if let Some(level) = ps.level[i].0.get(value) {
                    to.level = Some(*level);
                    true
                } else {
                    to.level = ps.level[i].1;
                    false
                }
            }
            Self::Logger => {
                to.logger = value.parse().ok();
                true
            }
            Self::Message => {
                to.message = Some(value);
                true
            }
            Self::Caller => {
                to.caller = value.parse().ok().map(|x| Caller::Text(x));
                true
            }
            Self::CallerFile => match &mut to.caller {
                None => {
                    to.caller = value.parse().ok().map(|x| Caller::FileLine(x, ""));
                    to.caller.is_some()
                }
                Some(Caller::FileLine(file, _)) => {
                    if let Some(value) = value.parse().ok() {
                        *file = value;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Self::CallerLine => match &mut to.caller {
                None => {
                    to.caller = Some(Caller::FileLine("", value.raw_str()));
                    true
                }
                Some(Caller::FileLine(_, line)) => match value {
                    RawValue::Number(value) => {
                        *line = value;
                        true
                    }
                    RawValue::String(_) => {
                        if let Some(value) = value.parse().ok() {
                            *line = value;
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                },
                _ => false,
            },
            Self::Nested(_) => false,
        }
    }

    #[inline]
    fn apply_ctx<'a>(
        &self,
        ps: &ParserSettings,
        value: RawValue<'a>,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
    ) -> bool {
        match *self {
            Self::Nested(nested) => match value {
                RawValue::Object(value) => {
                    let mut object = Object::default();
                    if value.parse_into(&mut object).is_ok() {
                        ps.blocks[nested].apply_each_ctx(ps, object.fields.iter(), to, ctx, false);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => self.apply(ps, value, to),
        }
    }

    #[inline]
    fn kind(&self) -> Option<FieldKind> {
        match self {
            Self::Time => Some(FieldKind::Time),
            Self::Level(_) => Some(FieldKind::Level),
            Self::Logger => Some(FieldKind::Logger),
            Self::Message => Some(FieldKind::Message),
            Self::Caller => Some(FieldKind::Caller),
            Self::CallerFile => Some(FieldKind::CallerFile),
            Self::CallerLine => Some(FieldKind::CallerLine),
            Self::Nested(_) => None,
        }
    }
}

// ---
