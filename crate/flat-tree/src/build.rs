// std imports
use std::result::Result;

// ---

pub trait Reserve {
    fn reserve(&mut self, _additional: usize) {}
}

pub trait Push: Reserve {
    type Value;

    fn push(self, value: Self::Value) -> Self;
}

// ---

// pub trait BuildE: Push + Sized {
//     type Child: BuildE<Value = Self::Value>;

//     fn build<E>(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Result<Self::Child, E>) -> Result<Self, E>;
// }

// impl<T: BuildE> Build for T {
//     type Child = T::Child;

//     #[inline]
//     fn build(self, value: Self::Value, f: impl FnOnce(Self::Child) -> Self::Child) -> Self {
//         unsafe { BuildE::build(self, value, |b| Ok::<_, ()>(f(b))).unwrap_unchecked() }
//     }
// }

// ---

// pub trait BuildComposite<B> {
//     type Result<Builder>: BuildCompositeResult<Builder>;

//     fn build(self, builder: B) -> Self::Result<B>;
// }

// impl<B, T> BuildComposite<B> for T
// where
//     T: FnOnce(B) -> B,
//     B: Build,
// {
//     type Result<Builder> = Builder;

//     #[inline]
//     fn build(self, builder: B) -> B {
//         self(builder)
//     }
// }

// impl<B, T, E> BuildComposite<B> for T
// where
//     T: FnOnce(B) -> Result<B, E>,
//     B: Build,
// {
//     type Result<Builder> = std::result::Result<Builder, E>;

//     #[inline]
//     fn build(self, builder: B) -> Self::Result<B> {
//         self(builder)
//     }
// }

// // ---

// pub trait BuildCompositeResult<Builder>: Into<Builder> {}

// impl<T> BuildCompositeResult<T> for T {}

/// A little helper trait that says:
/// "Given `B`, I can be called once with `B` to produce `Output`."
///

pub trait Build: Push + FromChild {
    fn build<C: BuildComposite<Self::Child>>(self, value: Self::Value, composite: C) -> C::Output;
}

pub trait FromChild: Sized {
    type Child: Build;

    fn from_child(child: Self::Child) -> Self;
}

/*
// A helper trait that says "Given B, I have an output type"
pub trait FnOnceExt<B> {
    type Output;
    fn call(self, b: B) -> Self::Output;
}

impl<F, B, O> FnOnceExt<B> for F
where
    F: FnOnce(B) -> O,
{
    type Output = O;

    fn call(self, b: B) -> O {
        self(b)
    }
}

pub trait BuildComposite<B: Build> {
    // This is now a *two-argument* GAT: we take `To` as well, because
    // we want to produce `To` from building `B`.
    type Result<To: FromChild>: Sized;

    fn build(self, builder: B) -> Self::Result<B>;
}

impl<B, T> BuildComposite<B> for T
where
    B: Build,        // or at least B: FromChild
    T: FnOnceExt<B>, // the closure's input is B
    T::Output: BuildCompositeResult<T::Output, B>,
{
    type Result<To: FromChild> = <T::Output as BuildCompositeResult<T::Output, To>>::Type;

    fn build(self, builder: B) -> Self::Result<B> {
        // 1. Call the closure => get T::Output
        let output = self.call(builder);
        // 2. Convert T::Output into the final type B
        <T::Output as BuildCompositeResult<T::Output, B>>::convert_output(output)
    }
}

/// "How do I map from some `From` type to `To` type in the build process?"
pub trait BuildCompositeResult<From, To> {
    type Type;

    fn convert_output(this: From) -> Self::Type;
}

// If the closure returns `Child` directly ...
impl<Child, Parent> BuildCompositeResult<Child, Parent> for Child
where
    Parent: FromChild<Child = Child>,
    Child: Build,
{
    type Type = Parent;
    fn convert_output(this: Child) -> Self::Type {
        Parent::from_child(this)
    }
}

// If the closure returns `Result<Child, E>`, ...
impl<Child, Parent, E> BuildCompositeResult<Result<Child, E>, Parent> for Result<Child, E>
where
    Parent: FromChild<Child = Child>,
    Child: Build,
{
    type Type = Result<Parent, E>;
    fn convert_output(this: Self) -> Self::Type {
        this.map(Parent::from_child)
    }
}

pub trait ExtractChildAndRest<Parent> {
    type Child: Build; // the actual child type
    type Rest; // leftover data, e.g. an error type or something else
    type FinalOutput; // the final result type after we build the child into `Parent`

    /// Extract (child, rest) from `self`.
    /// For a plain `Child`, rest might be `()`.
    /// For a `Result<Child, E>`, rest might be `E`.
    /// Or if you prefer, this could return `(Option<Child>, Rest)` or a `Result`.
    fn extract_parts(self) -> (Option<Self::Child>, Self::Rest);

    /// Combine a built `parent: Parent` with `rest: Self::Rest`
    /// to produce the final `Self::FinalOutput`.
    /// For a plain child, that final output might just be `Parent`.
    /// For a `Result<Child,E>`, it might be `Result<Parent, E>`.
    fn combine(parent: Option<Parent>, rest: Self::Rest) -> Self::FinalOutput;
}

impl<Child, Parent> ExtractChildAndRest<Parent> for Child
where
    Child: Build,
    Parent: FromChild<Child = Child>,
{
    type Child = Child;
    type Rest = (); // no leftover data
    type FinalOutput = Parent; // the final result is just the built parent

    fn extract_parts(self) -> (Option<Child>, ()) {
        (Some(self), ())
    }

    fn combine(parent: Option<Parent>, _rest: ()) -> Parent {
        parent.unwrap()
    }
}

impl<Child, Parent, E> ExtractChildAndRest<Parent> for Result<Child, E>
where
    Child: Build,
    Parent: FromChild<Child = Child>,
{
    type Child = Child;
    type Rest = Result<(), E>;
    type FinalOutput = Result<Parent, E>;

    fn extract_parts(self) -> (Option<Child>, Self::Rest) {
        // You could do something more robust, but to keep the shape the same,
        // maybe you store the "child or default" plus the actual error
        // Or you might do a "successful child" vs. "no child" approach:
        //  return (child, None) or (dummy_child, Some(e))...
        // But let's show a simpler approach with partial unwrapping:
        match self {
            Ok(child) => (Some(child), Ok(())),
            Err(e) => (None, e),
        }
    }

    fn combine(parent: Option<Parent>, rest: Self::Rest) -> Result<Parent, E> {
        rest.map(|_| parent.unwrap())
    }
}
*/

pub trait MapExtracted<Parent> {
    type Mapped;
    fn map_extracted(self) -> Self::Mapped;
}

impl<C, P> MapExtracted<P> for C
where
    C: Build,
    P: FromChild<Child = C>,
{
    type Mapped = P;
    fn map_extracted(self) -> Self::Mapped {
        P::from_child(self)
    }
}

impl<C, E, P> MapExtracted<P> for Result<C, E>
where
    C: Build,
    P: FromChild<Child = C>,
{
    type Mapped = Result<P, E>;
    fn map_extracted(self) -> Self::Mapped {
        self.map(|c| P::from_child(c))
    }
}

// Trait that builds a parent `B` from a closure T: FnOnceExt<B>.
pub trait BuildComposite<B: Build> {
    type Output;
    fn build(self, builder: B) -> Self::Output;
}

impl<B, T> BuildComposite<B> for T
where
    B: Build,
    T: FnOnceExt<B>,
    // closure returns T::Output
    // T::Output can be "child" or "Result<child,E>"
    T::Output: MapExtracted<B>,
{
    type Output = <T::Output as MapExtracted<B>>::Mapped;

    fn build(self, builder: B) -> Self::Output {
        let container = self.call(builder);
        container.map_extracted() // either yields B or Result<B,E>
    }
}

/// If I am a `Child` or a `Result<Child, E>`, I can yield a `Child` (or an error).
pub trait ExtractChild {
    /// The type of the “real child” we ultimately want to build from.
    type Child: Build;
    /// The error type if extraction fails. If there’s no real error concept
    /// (for the “naked child”), use something like `Infallible`.
    type Error;

    /// Extract the child, or fail with an error.
    fn try_extract_child(self) -> Result<Self::Child, Self::Error>;
}

// 1) If we’re just a `Child: Build`, extraction always succeeds with no error.
impl<C> ExtractChild for C
where
    C: Build, // If you want your "child" to also implement `Build`
{
    type Child = C;
    type Error = std::convert::Infallible; // or a custom "NoError" type

    fn try_extract_child(self) -> Result<C, Self::Error> {
        Ok(self)
    }
}

// 2) If we’re a `Result<Child, E>`, extraction is `self` itself.
impl<C, E> ExtractChild for Result<C, E>
where
    C: Build,
{
    type Child = C;
    type Error = E;

    fn try_extract_child(self) -> Result<C, E> {
        self
    }
}

pub trait FnOnceExt<B> {
    type Output;
    fn call(self, b: B) -> Self::Output;
}

impl<F, B, O> FnOnceExt<B> for F
where
    F: FnOnce(B) -> O,
{
    type Output = O;

    fn call(self, b: B) -> O {
        self(b)
    }
}
