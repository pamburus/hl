// std imports
use std::{collections::HashMap, env::var};

// third-party imports
use titlecase::titlecase;
use wildflower::Pattern;

// local imports
use super::{
    super::ast::{self, Build, Composite, OptIndex, Scalar},
    *,
};
use crate::{
    app::UnixTimestampUnit,
    model::{Caller, Level},
    settings::PredefinedFields,
    timestamp::Timestamp,
    types::FieldKind,
};

// ---

const MAX_DEPTH: usize = 64;

// ---

pub struct Builder<'s, 'c, 't, T, CP = ()> {
    core: Core<'s, CP>,
    ctx: Context<'c, 't>,
    target: T,
}

#[derive(Clone)]
struct Core<'s, CP> {
    settings: &'s Settings,
    block: Option<&'s SettingsBlock>,
    field: Option<(CP, FieldSettings)>,
    depth: usize,
}

struct Context<'c, 't> {
    record: &'c mut Record<'t>,
    pc: &'c mut PriorityController,
}

impl<'t, 's, 'c, T, CP> Builder<'s, 'c, 't, T, CP>
where
    T: Build<'t>,
{
    pub fn new(settings: &'s Settings, pc: &'c mut PriorityController, record: &'c mut Record<'t>, target: T) -> Self {
        Self {
            core: Core {
                settings,
                block: Some(&settings.blocks[0]),
                field: None,
                depth: 0,
            },
            ctx: Context { record, pc },
            target,
        }
    }

    fn into_inner(self) -> T::WithAttachment<Context<'c, 't>> {
        self.target.attach(self.ctx)
    }

    fn from_inner(core: Core<'s, CP>, target: T::WithAttachment<Context<'c, 't>>) -> Self {
        let (target, ctx) = target.detach();
        Self { core, ctx, target }
    }
}

impl<'t, 's, 'c, T> Build<'t> for Builder<'s, 'c, 't, T, T::Checkpoint>
where
    T: Build<'t>,
    't: 'c,
    T::Checkpoint: Clone,
{
    type Child = Builder<'s, 'c, 't, T::Child, T::Checkpoint>;
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<'s, 'c, 't, T::WithAttachment<V>, T::Checkpoint>;
    type WithoutAttachment = Builder<'s, 'c, 't, T::WithoutAttachment, T::Checkpoint>;
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(mut self, scalar: Scalar<'t>) -> Self {
        self.target = self.target.add_scalar(scalar);

        if let Some((checkpoint, settings)) = self.core.field.take() {
            if let Some(last) = self.target.first_node_index(&checkpoint).unfold() {
                let value = scalar.into();
                if settings.apply(self.core.settings, value, self.ctx.record).is_some() {
                    self.ctx.record.predefined.push(last).ok();
                    return self;
                }
            }
        }

        self
    }

    #[inline]
    fn add_composite<F>(mut self, composite: Composite<'t>, f: F) -> (Self, ast::Result<()>)
    where
        F: FnOnce(Self::Child) -> (Self::Child, ast::Result<()>),
    {
        let mut core = self.core.clone();

        match composite {
            Composite::Field(key) => {
                if let Some(block) = core.block {
                    if let Some((field, priority)) = block.fields.get(key.source()) {
                        match field.kind() {
                            FieldSettingsKind::Final(kind) => {
                                if !self.ctx.pc.prioritize(kind, *priority, || true) {
                                    return (self, Ok(()));
                                }
                                core.field = Some((self.target.checkpoint(), *field));
                            }
                            FieldSettingsKind::Nested(nested) => {
                                core.block = Some(&core.settings.blocks[nested]);
                            }
                        }
                    } else {
                        core.block = None;
                    }
                }
                if core.depth == 1 {
                    for pattern in &core.settings.ignore {
                        if pattern.matches(key.source()) {
                            return (self, Ok(()));
                        }
                    }
                }
            }
            Composite::Object => {
                if core.depth < MAX_DEPTH {
                    core.depth += 1;
                    core.field = None;
                } else {
                    log::error!("max depth exceeded, replaced an object with a string placeholder");
                    self.target = self
                        .target
                        .add_scalar(Scalar::String(ast::String::raw("<!max depth exceeded!>")));
                    return (self, Ok(()));
                }
            }
            Composite::Array => {
                if core.depth < MAX_DEPTH {
                    core.depth += 1;
                    core.field = None;
                } else {
                    log::error!("max depth exceeded, replaced an array with a string placeholder");
                    self.target = self
                        .target
                        .add_scalar(Scalar::String(ast::String::raw("<!max depth exceeded!>")));
                    return (self, Ok(()));
                }
            }
        }

        let self_core = self.core.clone();

        let (target, result) = self.into_inner().add_composite(composite, |target| {
            let (target, result) = f(Builder::from_inner(core, target));
            (target.into_inner(), result)
        });

        (Builder::from_inner(self_core, target), result)
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        self.target.checkpoint()
    }

    #[inline]
    fn first_node_index(&self, checkpoint: &Self::Checkpoint) -> OptIndex {
        self.target.first_node_index(checkpoint)
    }

    #[inline]
    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V> {
        Builder {
            core: self.core,
            ctx: self.ctx,
            target: self.target.attach(attachment),
        }
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, ast::AttachmentValue<Self::Attachment>) {
        let (target, value) = self.target.detach();
        (
            Builder {
                core: self.core,
                ctx: self.ctx,
                target,
            },
            value,
        )
    }
}

// ---

pub struct Settings {
    unix_ts_unit: Option<UnixTimestampUnit>,
    level: Vec<(HashMap<String, Level>, Option<Level>)>,
    blocks: Vec<SettingsBlock>,
    ignore: Vec<Pattern<String>>,
}

impl Settings {
    pub fn new(predefined: &PredefinedFields) -> Self {
        let mut result = Self {
            unix_ts_unit: None,
            level: Vec::new(),
            blocks: vec![SettingsBlock::default()],
            ignore: Vec::new(),
        };

        result.init(predefined);
        result
    }

    pub fn with_ignore<I>(mut self, ignore: I) -> Self
    where
        I: IntoIterator<Item: Into<String>>,
    {
        self.ignore = ignore.into_iter().map(|x| Pattern::new(x.into())).collect();
        self
    }

    pub fn with_unix_timestamp_unit(mut self, unix_ts_unit: Option<UnixTimestampUnit>) -> Self {
        self.unix_ts_unit = unix_ts_unit;
        self
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
            let Some(variant) = variant.resolve() else {
                continue;
            };

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
                    self.blocks.push(SettingsBlock::default());
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
    fn apply<'r, 's>(
        &self,
        key: &'s str,
        value: Value<'r, 's>,
        to: &'r mut Record<'s>,
        pc: &mut PriorityController,
    ) -> bool {
        self.blocks[0].apply(self, key, value, to, pc, true)
    }

    #[inline]
    fn apply_each<'r, 'a, 'i, I>(&self, items: I, to: &'r mut Record<'a>)
    where
        I: IntoIterator<Item = Field<'r, 'a>>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    #[inline]
    fn apply_each_ctx<'r, 'a, 'i, I>(&self, items: I, to: &'r mut Record<'a>, pc: &mut PriorityController)
    where
        I: IntoIterator<Item = Field<'r, 'a>>,
        'a: 'i,
    {
        for (i, field) in items.into_iter().enumerate() {
            if !self.apply(field.key, field.value, to, pc) {
                // to.hidden.set(i, true);
            }
        }
    }
}

impl Default for Settings {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

// ---

#[derive(Default, Debug)]
struct SettingsBlock {
    fields: HashMap<String, (FieldSettings, usize)>,
}

impl SettingsBlock {
    fn apply<'r, 's>(
        &self,
        ps: &Settings,
        key: &str,
        value: Value<'r, 's>,
        to: &'r mut Record<'s>,
        pc: &mut PriorityController,
        is_root: bool,
    ) -> bool {
        let done = match self.fields.get(key) {
            Some((field, priority)) => {
                let kind = field.kind();
                if let FieldSettingsKind::Final(kind) = kind {
                    pc.prioritize(kind, *priority, || field.apply(ps, value, to).is_some())
                } else {
                    field.apply_ctx(ps, value, to, pc);
                    false
                }
            }
            None => false,
        };
        if is_root && done {
            // to.predefined.push(Field::new(key, value)).ok();
        }

        !done
    }

    #[inline]
    fn apply_each_ctx<'r, 'a, 'i, I>(
        &self,
        ps: &Settings,
        fields: I,
        to: &'r mut Record<'a>,
        ctx: &mut PriorityController,
    ) where
        I: IntoIterator<Item = Field<'r, 'a>>,
        'a: 'i,
    {
        for field in fields {
            self.apply(ps, field.key, field.value, to, ctx, false);
        }
    }
}

// ---

#[derive(Default)]
pub struct PriorityController {
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
    fn prioritize<F: FnOnce() -> bool>(&mut self, kind: FieldKind, priority: usize, update: F) -> bool {
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
            update()
        } else {
            false
        }
    }

    #[inline]
    fn reset(&mut self) {
        *self = Default::default();
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
    fn apply<'r, 's>(&self, ps: &Settings, value: Value<'r, 's>, to: &'r mut Record<'s>) -> Option<()> {
        let as_text = |value: Value<'r, 's>| match value {
            Value::String(s) => s.source().into(),
            Value::Number(s) => s.into(),
            Value::Null => "null".into(),
            Value::Boolean(true) => "true".into(),
            Value::Boolean(false) => "false".into(),
            _ => None,
        };

        match *self {
            Self::Time => {
                let s = as_text(value)?;
                let s = if !s.is_empty() && s.as_bytes()[0] == b'"' {
                    &s[1..s.len() - 1]
                } else {
                    s
                };
                if !s.is_empty() {
                    let ts = Timestamp::new(s).with_unix_unit(ps.unix_ts_unit);
                    to.ts = Some(ts);
                    true
                } else {
                    false
                }
            }
            Self::Level(i) => {
                let s = as_text(value)?;
                if let Some(level) = ps.level[i].0.get(s) {
                    to.level = Some(*level);
                    true
                } else {
                    to.level = ps.level[i].1;
                    false
                }
            }
            Self::Logger => {
                to.logger = Some(as_text(value)?);
                true
            }
            Self::Message => match value.try_into() {
                Ok(value) => {
                    to.message = Some(value);
                    true
                }
                Err(_) => false,
            },
            Self::Caller => {
                to.caller.name = value.as_text()?.source();
                true
            }
            Self::CallerFile => {
                to.caller.file = value.as_text()?.source();
                true
            }
            Self::CallerLine => {
                let value = match value {
                    Value::Number(value) => value,
                    Value::String(s) => {
                        let s = s.source();
                        if s.is_empty() {
                            return None;
                        }
                        s
                    }
                    _ => return None,
                };

                to.caller.line = value;
                true
            }
            Self::Nested(_) => false,
        }
        .then(|| ())
    }

    #[inline]
    fn apply_ctx<'r, 's>(
        &self,
        ps: &Settings,
        value: Value<'r, 's>,
        to: &'r mut Record<'s>,
        ctx: &mut PriorityController,
    ) -> bool {
        match *self {
            Self::Nested(nested) => match value {
                Value::Object(value) => {
                    ps.blocks[nested].apply_each_ctx(ps, value.iter(), to, ctx);
                    true
                }
                _ => false,
            },
            _ => self.apply(ps, value, to).is_some(),
        }
    }

    #[inline]
    fn kind(&self) -> FieldSettingsKind {
        match self {
            Self::Time => FieldSettingsKind::Final(FieldKind::Time),
            Self::Level(_) => FieldSettingsKind::Final(FieldKind::Level),
            Self::Logger => FieldSettingsKind::Final(FieldKind::Logger),
            Self::Message => FieldSettingsKind::Final(FieldKind::Message),
            Self::Caller => FieldSettingsKind::Final(FieldKind::Caller),
            Self::CallerFile => FieldSettingsKind::Final(FieldKind::CallerFile),
            Self::CallerLine => FieldSettingsKind::Final(FieldKind::CallerLine),
            Self::Nested(index) => FieldSettingsKind::Nested(*index),
        }
    }
}

enum FieldSettingsKind {
    Final(FieldKind),
    Nested(usize),
}

// ---

#[cfg(test)]
mod tests;
