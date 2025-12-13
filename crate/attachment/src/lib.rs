use derive_more::{Deref, DerefMut};

// ---

pub trait Attach {
    type Attachment: Attachment;
    type WithAttachment<V>: Attach<Attachment = AttachmentChild<Self::Attachment, V>>;
    type WithoutAttachment: Attach<Attachment = AttachmentParent<Self::Attachment>>;

    fn attach<V>(self, attachment: V) -> Self::WithAttachment<V>;
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>);
}

pub trait Attachment {
    type Parent: Attachment;
    type Child<V>: Attachment<Value = V, Parent = Self>;
    type Value;

    fn join<V>(self, value: V) -> Self::Child<V>;
    fn split(self) -> (Self::Parent, Self::Value);
}

pub type AttachmentParent<A> = <A as Attachment>::Parent;
pub type AttachmentValue<A> = <A as Attachment>::Value;
pub type AttachmentChild<A, V> = <A as Attachment>::Child<V>;

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deref, DerefMut)]
pub struct WithAttachment<X, A: Attachment = NoAttachment>(
    #[deref]
    #[deref_mut]
    pub X,
    pub A,
);

impl<X> WithAttachment<X> {
    #[inline]
    pub fn new(x: X) -> Self {
        WithAttachment(x, NoAttachment)
    }
}

impl<R, E, A> WithAttachment<Result<R, E>, A>
where
    A: Attachment,
{
    #[inline]
    pub fn transpose(self) -> Result<WithAttachment<R, A>, WithAttachment<E, A>> {
        match self {
            WithAttachment(Ok(x), a) => Ok(WithAttachment(x, a)),
            WithAttachment(Err(x), a) => Err(WithAttachment(x, a)),
        }
    }
}

impl<X, A: Attachment> Attach for WithAttachment<X, A> {
    type Attachment = A;
    type WithAttachment<V> = WithAttachment<X, AttachmentChild<Self::Attachment, V>>;
    type WithoutAttachment = WithAttachment<X, AttachmentParent<Self::Attachment>>;

    #[inline]
    fn attach<V>(self, v: V) -> Self::WithAttachment<V> {
        WithAttachment(self.0, self.1.join(v))
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>) {
        let (a, v) = self.1.split();
        (WithAttachment(self.0, a), v)
    }
}

// ---

impl<R, E, A: Attachment> Attach for Result<WithAttachment<R, A>, WithAttachment<E, A>> {
    type Attachment = A;
    type WithAttachment<V> = Result<
        WithAttachment<R, AttachmentChild<Self::Attachment, V>>,
        WithAttachment<E, AttachmentChild<Self::Attachment, V>>,
    >;
    type WithoutAttachment = Result<
        WithAttachment<R, AttachmentParent<Self::Attachment>>,
        WithAttachment<E, AttachmentParent<Self::Attachment>>,
    >;

    #[inline]
    fn attach<V>(self, v: V) -> Self::WithAttachment<V> {
        match self {
            Ok(x) => Ok(x.attach(v)),
            Err(x) => Err(x.attach(v)),
        }
    }

    #[inline]
    fn detach(self) -> (Self::WithoutAttachment, AttachmentValue<Self::Attachment>) {
        match self {
            Ok(r) => {
                let (r, v) = r.detach();
                (Ok(r), v)
            }
            Err(e) => {
                let (e, v) = e.detach();
                (Err(e), v)
            }
        }
    }
}

// ---

pub trait InitialAttachment {
    type WithAttachment<V>: Attach<Attachment = PlainAttachment<NoAttachment, V>>;

    fn with_attachment<V>(self, attachment: V) -> Self::WithAttachment<V>;
}

impl<R, E> InitialAttachment for Result<R, E> {
    type WithAttachment<V> = Result<
        WithAttachment<R, PlainAttachment<NoAttachment, V>>,
        WithAttachment<E, PlainAttachment<NoAttachment, V>>,
    >;

    #[inline]
    fn with_attachment<V>(self, attachment: V) -> Self::WithAttachment<V> {
        match self {
            Ok(x) => Ok(WithAttachment(x, NoAttachment).attach(attachment)),
            Err(x) => Err(WithAttachment(x, NoAttachment).attach(attachment)),
        }
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct NoAttachment;

impl Attachment for NoAttachment {
    type Parent = NoAttachment;
    type Child<V> = PlainAttachment<Self, V>;
    type Value = ();

    #[inline]
    fn join<V>(self, value: V) -> Self::Child<V> {
        PlainAttachment { parent: self, value }
    }

    #[inline]
    fn split(self) -> (Self::Parent, Self::Value) {
        (self, ())
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PlainAttachment<P, V> {
    parent: P,
    value: V,
}

impl<P, V> Attachment for PlainAttachment<P, V>
where
    P: Attachment,
{
    type Parent = P;
    type Child<V2> = PlainAttachment<Self, V2>;
    type Value = V;

    #[inline]
    fn join<V2>(self, value: V2) -> Self::Child<V2> {
        PlainAttachment { parent: self, value }
    }

    #[inline]
    fn split(self) -> (Self::Parent, Self::Value) {
        (self.parent, self.value)
    }
}

// ---

#[cfg(test)]
mod tests;
