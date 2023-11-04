pub enum Expr {
    Or(Box<Expr>, Box<Expr>),
    ExprP1(ExprP1),
}

pub enum ExprP1 {
    And(Box<ExprP1>, Box<ExprP1>),
    Unary(Unary),
}

pub enum Unary {
    Not(Box<Unary>),
    Primary(Primary),
}

pub enum Primary {
    Expr(Box<Expr>),
    Term(Term),
}

pub enum Term {
    LevelFilter(LevelFilter),
    FieldFilter(FieldFilter),
}

pub enum LevelFilter {
    Is(Level),
}

pub struct FieldFilter {
    key: String,
    value: String,
    op: StringOp,
}

pub enum StringOp {
    Equal,
    Like,
    RegexMatch,
    NotEqual,
    NotLike,
    NotRegexMatch,
}
