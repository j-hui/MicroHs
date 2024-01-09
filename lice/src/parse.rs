//! Combinator file parser.
use crate::comb::{CombFile, Expr, Index, Label, Prim, Program, NIL_INDEX};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alphanumeric1, char, digit1, multispace0},
    combinator::{map_res, opt, recognize, verify},
    multi::{fold_many0, many1_count},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair},
    Finish, IResult, Parser,
};
use std::str::FromStr;

/// The result of parsing. A parser monad, even. Typedef'd here for convenience.
type Parse<'a, T> = IResult<&'a str, T>;

fn uinteger(i: &str) -> Parse<usize> {
    map_res(digit1, |s: &str| s.parse::<usize>()).parse(i)
}

fn integer(i: &str) -> Parse<i64> {
    map_res(recognize(preceded(opt(char('-')), digit1)), |s: &str| {
        s.parse::<i64>()
    })
    .parse(i)
}

fn string_literal(input: &str) -> Parse<String> {
    enum StringFragment<'a> {
        Literal(&'a str),
        EscapedChar(char),
    }

    let literal = verify(is_not("\"\\"), |s: &str| !s.is_empty());
    let escaped_char = preceded(
        char('\\'),
        map_res(digit1, |s: &str| s.parse::<u8>()).map(|n| n as char),
    );

    let build_string = fold_many0(
        alt((
            // The `map` combinator runs a parser, then applies a function to the output
            // of that parser.
            literal.map(StringFragment::Literal),
            escaped_char.map(StringFragment::EscapedChar),
        )),
        // Our init value, an empty string
        String::new,
        // Our folding function. For each fragment, append the fragment to the
        // string.
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedChar(c) => string.push(c),
            }
            string
        },
    );
    delimited(char('"'), build_string, char('"')).parse(input)
}

impl CombFile {
    fn parse(i: &str) -> Parse<Self> {
        let i = multispace0(i)?.0;
        let (i, version) = preceded(char('v'), separated_pair(uinteger, char('.'), uinteger))(i)?;
        let i = multispace0(i)?.0;
        let (i, size) = uinteger(i)?;
        let i = multispace0(i)?.0;
        let (i, program) = Program::parse(i, size)?;

        Ok((
            i,
            Self {
                version,
                size,
                program,
            },
        ))
    }
}

impl Program {
    fn parse(i: &str, size: usize) -> Parse<Self> {
        let mut p = Self {
            root: NIL_INDEX,
            body: Vec::new(),
            defs: Vec::new(),
        };
        p.defs.resize(size, NIL_INDEX);

        let (i, root) = p.parse_expr(i)?;
        p.root = root;

        for (label, &def) in p.defs.iter().enumerate() {
            // TODO: convert to actual error? or just throw hands like we do now
            assert!(def != NIL_INDEX, "label {label} is not initialized!");
        }

        Ok((i, p))
    }

    fn parse_expr<'i>(&mut self, i: &'i str) -> Parse<'i, Index> {
        let prim_token = recognize(many1_count(alt((
            // Characters that possibly appear in a primitive identifier
            alphanumeric1,
            tag("'"),
            tag("."),
            tag("+"),
            tag("-"),
            tag("*"),
            tag("/"),
            tag("="),
            tag("<"),
            tag(">"),
            tag("&"),
            tag("|"),
            tag("!"),
        ))));

        let i = multispace0(i)?.0;
        let (i, c) = if let Ok((i, (f, l, a))) = self.parse_app(i) {
            (i, (Expr::App(f, l, a)))
        } else if let Ok((i, (sz, arr))) = self.parse_array(i) {
            (i, (Expr::Array(sz, arr)))
        } else {
            alt((
                preceded(char('&'), double).map(Expr::Float),
                preceded(char('#'), integer).map(Expr::Int),
                preceded(char('_'), uinteger).map(Expr::Ref),
                string_literal.map(Expr::String),
                preceded(char('!'), string_literal).map(Expr::Tick),
                preceded(char('^'), alphanumeric1) // NOTE: this accepts identifiers like ^1piece
                    .map(String::from)
                    .map(Expr::Ffi),
                prim_token.map(|s| {
                    if let Ok(p) = Prim::from_str(s) {
                        Expr::Prim(p)
                    } else {
                        Expr::Unknown(s.to_string())
                    }
                }),
            ))
            .parse(i)?
        };

        let i = multispace0(i)?.0;

        let index = self.body.len();
        self.body.push(c);

        Ok((i, index))
    }

    fn parse_app<'i>(&mut self, i: &'i str) -> Parse<'i, (Index, Option<Label>, Index)> {
        let i = char('(')(i)?.0;
        let i = multispace0(i)?.0;
        let (i, f) = self.parse_expr(i)?;
        let i = multispace0(i)?.0;
        let (i, label) = opt(preceded(char(':'), uinteger))(i)?; // possible :def
        let i = multispace0(i)?.0;
        let (i, a) = self.parse_expr(i)?;
        let i = multispace0(i)?.0;
        let i = char(')')(i)?.0;

        if let Some(label) = label {
            self.defs[label] = a;
        }

        Ok((i, (f, label, a)))
    }

    fn parse_array<'i>(&mut self, i: &'i str) -> Parse<'i, (usize, Vec<Index>)> {
        let i = char('[')(i)?.0;
        let i = multispace0(i)?.0;
        let (i, sz) = uinteger(i)?;

        let mut v = Vec::new();
        v.reserve_exact(sz);

        let mut ii = i; // Keep around input slice between iters of this awkward loop
        loop {
            let i = ii;
            let i = multispace0(i)?.0;
            if let (i, Some(_)) = opt(char(']'))(i)? {
                // assert that the vector is large enough?
                return Ok((i, (sz, v)));
            }
            let (i, c) = self.parse_expr(i)?;
            v.push(c);
            ii = i;
        }
    }
}

impl FromStr for CombFile {
    type Err = ();

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        if let Ok((_, c)) = CombFile::parse(s).finish() {
            Ok(c)
        } else {
            Err(())
        }
    }
}
