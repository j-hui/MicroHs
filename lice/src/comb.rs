//! Combinators

use parse_display::{Display, FromStr};

pub use parse_display::ParseError;

pub type Index = usize;
pub type Label = usize;
pub type Int = i64;
pub type Float = f64;

#[derive(Debug, Clone)]
pub struct CombFile {
    // (Major, minor)
    pub version: (usize, usize),

    /// (Maximum) number of definitions.
    pub size: usize,

    /// Program embedded in the comb file.
    pub program: Program,
}

#[derive(Debug, Clone)]
pub struct Program {
    // The root combinator expression
    pub root: Index,

    /// `Map<Index, Expr>`
    pub body: Vec<Expr>,

    /// `Map<Label, Index>`
    pub defs: Vec<Index>,
}

#[derive(Debug, Clone, Display)]
pub enum Expr {
    /// Application of two expressions, with possible definition label: i.e., `(func [:label] arg)`.
    #[display("@")]
    App(Index, Option<Label>, Index),
    /// Floating point literal, i.e., `&float`.
    #[display("&{0}")]
    Float(Float),
    /// Integer literal, possibly negative, i.e., `#[-]int`.
    #[display("#{0}")]
    Int(Int),
    /// Fixed size array of expressions, i.e., `[size arr]`.
    #[display("[{0}]")]
    Array(usize, Vec<Index>),
    /// Reference to some labeled definition, i.e., `_label`.
    #[display("*")]
    Ref(Label),
    /// String literal, i.e., `"str"`
    #[display("{0:#?}")]
    String(String),
    /// Tick mark, i.e., `!"tick"`.
    #[display("!{0:#?}")]
    Tick(String),
    /// FFI symbol, i.e., `^symbol`.
    #[display("^{0}")]
    Ffi(String),
    /// Combinators and other primitives, e.g., `S` or `IO.>>=`.
    #[display("{0}")]
    Prim(Prim),
    /// Default case. Shouldn't appear, but you know, life happens.
    #[display("?!{0}")]
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
#[display("{0}")]
pub enum Prim {
    Combinator(Combinator),
    BuiltIn(BuiltIn),
    Arith(Arith),
    Pointer(Pointer),
    IO(IO),
    FArith(FArith),
    Array(Array),
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum Combinator {
    S,
    K,
    I,
    B,
    C,
    A,
    Y,
    #[display("S'")]
    SS,
    #[display("B'")]
    BB,
    #[display("C'")]
    CC,
    P,
    R,
    O,
    U,
    Z,
    K2,
    K3,
    K4,
    #[display("C'B")]
    CCB,
}

impl Combinator {
    pub fn arity(&self) -> usize {
        match self {
            Combinator::S => 3,
            Combinator::K => 2,
            Combinator::I => 1,
            Combinator::B => 3,
            Combinator::C => 3,
            Combinator::A => 2,
            Combinator::Y => 1,
            Combinator::SS => 4,
            Combinator::BB => 4,
            Combinator::CC => 4,
            Combinator::P => 3,
            Combinator::R => 3,
            Combinator::O => 3,
            Combinator::U => 2,
            Combinator::Z => 3,
            Combinator::K2 => 3,
            Combinator::K3 => 4,
            Combinator::K4 => 5,
            Combinator::CCB => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum BuiltIn {
    #[display("error")]
    Error,
    #[display("noDefault")]
    NoDefault,
    #[display("noMatch")]
    NoMatch,
    #[display("seq")]
    Seq,
    #[display("equal")]
    Equal,
    #[display("sequal")]
    SEqual,
    #[display("compare")]
    Compare,
    #[display("scmp")]
    SCmp,
    #[display("icmp")]
    ICmp,
    #[display("rnf")]
    Rnf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum Arith {
    #[display("+")]
    Add,
    #[display("-")]
    Sub,
    #[display("*")]
    Mul,
    #[display("quot")]
    Quot,
    #[display("rem")]
    Rem,
    #[display("subtract")]
    Subtract,
    #[display("uquot")]
    UQuot,
    #[display("urem")]
    URem,
    #[display("neg")]
    Neg,
    #[display("and")]
    And,
    #[display("or")]
    Or,
    #[display("xor")]
    Xor,
    #[display("inv")]
    Inv,
    #[display("shl")]
    Shl,
    #[display("shr")]
    Shr,
    #[display("ashr")]
    AShr,
    #[display("eq")]
    Eq,
    #[display("ne")]
    Ne,
    #[display("lt")]
    Lt,
    #[display("le")]
    Le,
    #[display("gt")]
    Gt,
    #[display("ge")]
    Ge,
    #[display("u<")]
    ULt,
    #[display("u<=")]
    ULe,
    #[display("u>")]
    UGt,
    #[display("u>=")]
    UGe,
    #[display("toInt")]
    ToInt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum Pointer {
    #[display("p==")]
    PEq,
    #[display("pnull")]
    PNull,
    #[display("p+")]
    PAdd,
    #[display("p=")]
    PSub,
    #[display("toPtr")]
    ToPtr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum IO {
    #[display("IO.>>=")]
    Bind,
    #[display("IO.>>")]
    Then,
    #[display("IO.return")]
    Return,
    #[display("IO.serialize")]
    Serialize,
    #[display("IO.deserialize")]
    Deserialize,
    #[display("IO.stdin")]
    StdIn,
    #[display("IO.stdout")]
    StdOut,
    #[display("IO.stderr")]
    StdErr,
    #[display("IO.getArgs")]
    GetArgs,
    #[display("IO.performIO")]
    PerformIO,
    #[display("IO.getTimeMilli")]
    GetTimeMilli,
    #[display("IO.print")]
    Print,
    #[display("IO.catch")]
    Catch,
    #[display("dynsym")]
    DynSym,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum FArith {
    #[display("f+")]
    FAdd,
    #[display("f-")]
    FSub,
    #[display("f*")]
    FMul,
    #[display("f/")]
    FDiv,
    #[display("fneg")]
    FNeg,
    /// Integer to floating point conversion.
    #[display("itof")]
    IToF,
    #[display("f==")]
    Feq,
    #[display("f/=")]
    FNe,
    #[display("f<")]
    FLt,
    #[display("f<=")]
    FLe,
    #[display("f>")]
    FGt,
    #[display("f>=")]
    FGe,
    #[display("fshow")]
    FShow,
    #[display("fread")]
    FRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromStr)]
pub enum Array {
    #[display("A.alloc")]
    Alloc,
    #[display("A.size")]
    Size,
    #[display("A.read")]
    Read,
    #[display("A.write")]
    Write,
    #[display("A.==")]
    Eq,
    #[display("newCAStringLen")]
    NewCAStringLen,
    #[display("peekCAString")]
    PeekCAString,
    #[display("peekCAStringLen")]
    PeekCAStringLen,
}

impl std::fmt::Display for CombFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "v{}.{}\n{}\n{}",
            self.version.0, self.version.1, self.size, self.program
        )
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_expr(out, self.root)
    }
}

impl Program {
    fn fmt_expr(&self, out: &mut std::fmt::Formatter<'_>, idx: Index) -> std::fmt::Result {
        match self.body.get(idx).ok_or(std::fmt::Error)? {
            Expr::App(f, l, a) => {
                write!(out, "(")?;
                self.fmt_expr(out, *f)?;
                write!(out, " ")?;
                if let Some(l) = l {
                    write!(out, ":{l} ")?;
                }
                self.fmt_expr(out, *a)?;
                write!(out, ")")
            }
            Expr::Array(sz, arr) => {
                // assert!(sz == arr.len());
                write!(out, "[{sz}")?;
                for a in arr {
                    write!(out, " ")?;
                    self.fmt_expr(out, *a)?;
                }
                write!(out, "]")
            }
            Expr::String(s) => {
                write!(out, "\"")?;
                for c in s.chars() {
                    if c.is_ascii_graphic() || c == ' ' {
                        write!(out, "{c}")?;
                    } else {
                        write!(out, "\\{}", c as usize)?;
                    }
                }
                write!(out, "\"")
            }
            Expr::Tick(s) => {
                write!(out, "!\"")?;
                for c in s.chars() {
                    if c.is_ascii_graphic() || c == ' ' {
                        write!(out, "{c}")?;
                    } else {
                        write!(out, "\\{}", c as usize)?;
                    }
                }
                write!(out, "\"")
            }
            expr => {
                // `Expr's derived Display implementation is sufficient
                write!(out, "{}", expr)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn display_prims() {
        // Spot-check some random primitives
        assert_eq!(Combinator::A.to_string(), "A");
        assert_eq!(Combinator::SS.to_string(), "S'");
        assert_eq!(Combinator::CCB.to_string(), "C'B");
        assert_eq!(Combinator::K2.to_string(), "K2");

        assert_eq!(Prim::Combinator(Combinator::A).to_string(), "A");
        assert_eq!(Prim::Combinator(Combinator::SS).to_string(), "S'");
        assert_eq!(Prim::Combinator(Combinator::CCB).to_string(), "C'B");
        assert_eq!(Prim::Combinator(Combinator::K2).to_string(), "K2");

        assert_eq!(BuiltIn::Error.to_string(), "error");
        assert_eq!(BuiltIn::NoDefault.to_string(), "noDefault");

        assert_eq!(Arith::Add.to_string(), "+");
        assert_eq!(Arith::Neg.to_string(), "neg");
        assert_eq!(Arith::ULe.to_string(), "u<=");
        assert_eq!(Arith::ToInt.to_string(), "toInt");

        assert_eq!(IO::Bind.to_string(), "IO.>>=");
        assert_eq!(IO::Return.to_string(), "IO.return");
        assert_eq!(IO::StdOut.to_string(), "IO.stdout");
        assert_eq!(IO::PerformIO.to_string(), "IO.performIO");

        assert_eq!(Array::NewCAStringLen.to_string(), "newCAStringLen");
    }

    #[test]
    fn parse_prims() {
        assert_eq!(Ok(Combinator::A), "A".parse());
        assert_eq!(Ok(Combinator::SS), "S'".parse());
        assert_eq!(Ok(Combinator::CCB), "C'B".parse());
        assert_eq!(Ok(Combinator::K2), "K2".parse());

        assert_eq!(Ok(Prim::Combinator(Combinator::A)), "A".parse());
        assert_eq!(Ok(Prim::Combinator(Combinator::SS)), "S'".parse());
        assert_eq!(Ok(Prim::Combinator(Combinator::CCB)), "C'B".parse());
        assert_eq!(Ok(Prim::Combinator(Combinator::K2)), "K2".parse());

        assert_eq!(Ok(Arith::Add), "+".parse());
        assert_eq!(Ok(Arith::Neg), "neg".parse());
        assert_eq!(Ok(Arith::ULe), "u<=".parse());
        assert_eq!(Ok(Arith::ToInt), "toInt".parse());

        assert!(Combinator::from_str("u<=").is_err());
        assert!(Arith::from_str("C'B").is_err());
    }

    #[test]
    fn display_program() {
        // An arbitrarily constructed test case, deliberately featuring:
        // - at least one of each type of expr
        // - a root that doesn't have the last index
        // - negative floating and integer literals
        // - two app exprs that point to the same expr (without indirection)
        // - an otherwise confounding tree structure
        let p = CombFile {
            version: (6, 19),
            size: 1,
            program: Program {
                root: 10,
                body: vec![
                    /* 0 */ Expr::Prim(Prim::Combinator(Combinator::K4)),
                    /* 1 */ Expr::Prim(Prim::Combinator(Combinator::CCB)),
                    /* 2 */ Expr::Prim(Prim::IO(IO::Bind)),
                    /* 3 */ Expr::Int(-42),
                    /* 4 */ Expr::Float(-4.2),
                    /* 5 */ Expr::String("Hello world!\r\n".to_string()),
                    /* 6 */ Expr::Tick("Lyme's".to_string()),
                    /* 7 */ Expr::Ffi("fork".to_string()),
                    /* 8 */ Expr::Array(5, vec![3, 4, 5, 6, 7]),
                    /* 9 */ Expr::Ffi("UNREACHABLE!".to_string()),
                    /* 10 */ Expr::App(2, Some(0), 13),
                    /* 11 */ Expr::App(10, None, 14),
                    /* 12 */ Expr::App(1, None, 2),
                    /* 13 */ Expr::App(8, None, 0),
                    /* 14 */ Expr::App(12, None, 15),
                    /* 15 */ Expr::Ref(0),
                ],
                defs: vec![],
            },
        };

        assert_eq!(
            p.to_string(),
            r#"v6.19
1
(IO.>>= :0 ([5 #-42 &-4.2 "Hello world!\13\10" !"Lyme's" ^fork] K4))"#
        );
    }
}
