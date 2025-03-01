// std imports
use std::{collections::HashMap, env::var};

// third-party imports
use titlecase::titlecase;
use wildflower::Pattern;

// workspace imports
use log_ast::ast::{Build, Value};

// local imports
use super::{
    super::ast::{self, Composite, OptIndex, Scalar},
    Record,
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

pub struct Builder<'s, 'c, T, CP = ()> {
    core: Core<'s, CP>,
    ctx: Context<'c>,
    target: T,
}

#[derive(Clone)]
struct Core<'s, CP> {
    settings: &'s Settings,
    source: &'s str,
    block: Option<&'s SettingsBlock>,
    field: Option<(CP, FieldSettings)>,
    depth: usize,
}

struct Context<'c> {
    record: &'c mut Record,
    pc: &'c mut PriorityController,
}

impl<'s, 'c, T, CP> Builder<'s, 'c, T, CP>
where
    T: Build,
{
    pub fn new(
        settings: &'s Settings,
        source: &'s str,
        pc: &'c mut PriorityController,
        record: &'c mut Record,
        target: T,
    ) -> Self {
        Self {
            core: Core {
                settings,
                source,
                block: Some(&settings.blocks[0]),
                field: None,
                depth: 0,
            },
            ctx: Context { record, pc },
            target,
        }
    }

    fn into_inner(self) -> T::WithAttachment<Context<'c>> {
        self.target.attach(self.ctx)
    }

    fn from_inner(core: Core<'s, CP>, target: T::WithAttachment<Context<'c>>) -> Self {
        let (target, ctx) = target.detach();
        Self { core, ctx, target }
    }
}

impl<'s, 'c, T> Build for Builder<'s, 'c, T, T::Checkpoint>
where
    T: Build,
    T::Checkpoint: Clone,
{
    type Attachment = T::Attachment;
    type WithAttachment<V> = Builder<'s, 'c, T::WithAttachment<V>, T::Checkpoint>;
    type WithoutAttachment = Builder<'s, 'c, T::WithoutAttachment, T::Checkpoint>;
    type Checkpoint = T::Checkpoint;

    #[inline]
    fn add_scalar(mut self, scalar: Scalar) -> Self {
        self.target = self.target.add_scalar(scalar);

        if let Some((checkpoint, settings)) = self.core.field.take() {
            if let Some(last) = self.target.first_node_index(&checkpoint).unfold() {
                let value = scalar.into();
                if settings.apply(self.core.settings, value, self.ctx.record).is_some() {
                    self.ctx.record.hidden.push(last).ok();
                    return self;
                }
            }
        }

        self
    }

    #[inline]
    fn add_composite<E, F>(mut self, composite: Composite, f: F) -> Result<Self, (E, Self)>
    where
        F: FnOnce(Self) -> Result<Self, (E, Self)>,
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
    fn rollback(&mut self, checkpoint: &Self::Checkpoint) {
        self.target.rollback(checkpoint);
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
    fn apply<'r, 's>(&self, key: &'s str, value: Value, to: &'r mut Record, pc: &mut PriorityController) -> bool {
        self.blocks[0].apply(self, key, value, to, pc, true)
    }

    #[inline]
    fn apply_each<'r, 'a, 'i, I>(&self, items: I, to: &'r mut Record)
    where
        I: IntoIterator<Item = Field<'r>>,
        'a: 'i,
    {
        let mut pc = PriorityController::default();
        self.apply_each_ctx(items, to, &mut pc);
    }

    #[inline]
    fn apply_each_ctx<'r, 'a, 'i, I>(&self, items: I, to: &'r mut Record, pc: &mut PriorityController)
    where
        I: IntoIterator<Item = Field<'r>>,
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
        value: Value,
        to: &'r mut Record,
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
    fn apply_each_ctx<'r, 'a, 'i, I>(&self, ps: &Settings, fields: I, to: &'r mut Record, ctx: &mut PriorityController)
    where
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
    fn apply<'r, 's>(&self, ps: &Settings, value: Value<'r, 's>, to: &'r mut Record) -> Option<()> {
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
                let s = if s.as_bytes()[0] == b'"' { &s[1..s.len() - 1] } else { s };
                let ts = Timestamp::new(s).with_unix_unit(ps.unix_ts_unit);
                to.ts = Some(ts);
                true
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
                to.caller = Some(Caller::Text(as_text(value)?));
                true
            }
            Self::CallerFile => match &mut to.caller {
                None => {
                    to.caller = Some(Caller::FileLine(as_text(value)?, ""));
                    true
                }
                Some(Caller::FileLine(file, _)) => {
                    *file = as_text(value)?;
                    true
                }
                _ => false,
            },
            Self::CallerLine => match &mut to.caller {
                None => {
                    to.caller = Some(Caller::FileLine("", as_text(value)?));
                    true
                }
                Some(Caller::FileLine(_, line)) => {
                    *line = as_text(value)?;
                    true
                }
                _ => false,
            },
            Self::Nested(_) => false,
        }
        .then(|| ())
    }

    #[inline]
    fn apply_ctx<'r, 's>(&self, ps: &Settings, value: Value, to: &'r mut Record, ctx: &mut PriorityController) -> bool {
        match *self {
            Self::Nested(nested) => match value {
                Value::Composite(Composite::Object(value)) => {
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
mod tests {
    use crate::format::json;
    use logos::Logos;

    use super::*;

    #[test]
    fn test_builder() {
        let settings = Settings::default();
        let mut container = ast::Container::new();
        let mut pc = PriorityController::default();
        let mut record = Record::default();
        let b = Builder::new(&settings, &mut pc, &mut record, container.metaroot());
        b.add_scalar(Scalar::Bool(true))
            .add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(false)), Ok(())))
            .1
            .unwrap();
        assert_eq!(container.nodes().len(), 3);
    }

    #[test]
    fn test_builder_json() {
        let mut container = ast::Container::new();
        let mut pc = PriorityController::default();
        let settings = Settings::new(&PredefinedFields::default()).with_ignore(["kubernetes", "agent"]);
        let mut record = Record::default();
        json::parse_all_into(
            &mut json::Token::lexer(KIBANA_REC_1),
            Builder::new(&settings, &mut pc, &mut record, container.metaroot()),
        )
        .1
        .unwrap();

        assert_eq!(container.roots().len(), 1);
        assert_eq!(container.roots().iter().next().unwrap().children().len(), 22);
        assert_eq!(container.nodes().len(), 57);

        println!("{:?}", container);

        record.ast = container;
        assert_eq!(record.hidden.len(), 5);
        assert_eq!(record.fields_for_search().into_iter().count(), 22);
        assert_eq!(record.fields().into_iter().count(), 17);
    }

    const KIBANA_REC_1: &str = r#"{"@timestamp":"2021-06-20T00:00:00.393Z","@version":"1","agent":{"ephemeral_id":"30ca3b53-1ef6-4699-8728-7754d1698a01","hostname":"as-rtrf-fileboat-ajjke","id":"1a9b51ef-ffbe-420e-a92c-4f653afff5aa","type":"fileboat","version":"7.8.3"},"koent-id":"1280e812-654f-4d04-a4f8-e6b84079920a","anchor":"oglsaash","caller":"example/demo.go:200","dc_name":"as-rtrf","ecs":{"version":"1.0.0"},"host":{"name":"as-rtrf-fileboat-ajjke"},"input":{"type":"docker"},"kubernetes":{"container":{"name":"some-segway"},"labels":{"app":"some-segway","component":"some-segway","pod-template-hash":"756d998476","release":"as-rtrf-some-segway","subcomponent":"some-segway"},"namespace":"as-rtrf","node":{"name":"as-rtrf-k8s-kube-node-vm01"},"pod":{"name":"as-rtrf-some-segway-platform-756d998476-jz4jm","uid":"9d445b65-fbf7-4d94-a7f4-4dbb7753d65c"},"replicaset":{"name":"as-rtrf-some-segway-platform-756d998476"}},"level":"info","localTime":"2021-06-19T23:59:58.450Z","log":{"file":{"path":"/var/lib/docker/containers/38a5db8e-45dc-4c33-b38a-6f8a9794e894/74f0afa4-3003-4119-8faf-19b97d27272e/f2b3fc41-4d71-4fe3-a0c4-336eb94dbcca/80c2448b-7806-404e-8e3a-9f88c30a0496-json.log"},"offset":34009140},"logger":"deep","msg":"io#2: io#1rq#8743: readfile = {.offset = 0x4565465000, .length = 4096, .lock_id = dc0cecb7-5179-4daa-9421-b2548b5ed7bf}, xxaao_client = 1","server-uuid":"0a1bec7f-a252-4ff6-994a-1fbdca318d6d","slot":2,"stream":"stdout","task-id":"1a632cba-8480-4644-93f2-262bc0c13d04","tenant-id":"40ddb7cf-ce50-41e4-b994-408e393355c0","time":"2021-06-20T00:00:00.393Z","ts":"2021-06-19T23:59:58.449489225Z","type":"k8s_containers_logs","unit":"0"}"#;
}
