// std imports
use std::collections::HashMap;

// third-party imports
use derive_more::{Deref, DerefMut};
use titlecase::titlecase;
use wildflower::Pattern;

// local imports
use super::{
    super::ast::{self, BuildExt, Composite, Scalar},
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

#[derive(Deref, DerefMut)]
pub struct Builder<'s, T> {
    #[deref]
    #[deref_mut]
    core: Core<'s>,
    target: T,
}

#[derive(Clone, Copy)]
struct Core<'s> {
    settings: &'s Settings,
}

impl<'s, T> Builder<'s, T>
where
    T: BuildExt<'s>,
{
    pub fn new(settings: &'s Settings, target: T) -> Self {
        Self {
            core: Core { settings },
            target,
        }
    }

    fn child(core: Core<'s>, target: T::Child) -> Builder<'s, T::Child> {
        Builder { core, target }
    }
}

impl<'s, T> BuildExt<'s> for Builder<'s, T>
where
    T: BuildExt<'s>,
{
    type Child = Builder<'s, T::Child>;

    #[inline]
    fn add_scalar(mut self, scalar: Scalar<'s>) -> Self {
        self.target = self.target.add_scalar(scalar);
        self
    }

    #[inline]
    fn add_composite(
        mut self,
        composite: Composite<'s>,
        f: impl FnOnce(Self::Child) -> ast::Result<Self::Child>,
    ) -> ast::Result<Self> {
        self.target = self
            .target
            .add_composite(composite, |target| Ok(f(Self::child(self.core, target))?.target))?;
        Ok(self)
    }
}

// ---

pub struct Settings {
    unix_ts_unit: Option<UnixTimestampUnit>,
    level: Vec<(HashMap<String, Level>, Option<Level>)>,
    blocks: Vec<ParserSettingsBlock>,
    ignore: Vec<Pattern<String>>,
}

impl Settings {
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

impl Default for Settings {
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
        ps: &Settings,
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
        ps: &Settings,
        items: I,
        to: &mut Record<'a>,
        ctx: &mut PriorityController,
        is_root: bool,
    ) where
        I: IntoIterator<Item = &'i (&'a str, Value<'a>)>,
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
    fn apply<'a>(&self, ps: &Settings, value: RawValue<'a>, to: &mut Record<'a>) -> bool {
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
        ps: &Settings,
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
