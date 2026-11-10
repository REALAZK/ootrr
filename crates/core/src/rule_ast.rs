use crate::error::{Error as E, Res};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Floor,
    Rem,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    Name(String),
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Tuple(Vec<Ast>),
    List(Vec<Ast>),
    Call(String, Vec<Ast>),
    Index(String, Box<Ast>),
    Not(Box<Ast>),
    Neg(Box<Ast>),
    And(Vec<Ast>),
    Or(Vec<Ast>),
    Cmp(Box<Ast>, Vec<(Cmp, Ast)>),
    Bin(Bop, Box<Ast>, Box<Ast>),
    If(Box<Ast>, Box<Ast>, Box<Ast>),
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Name(String),
    Str(String),
    Int(i64),
    Float(f64),
    Sym(&'static str),
    End,
}

const SYMS2: [&str; 5] = ["==", "!=", "<=", ">=", "//"];
const SYMS1: [&str; 12] = ["(", ")", "[", "]", ",", "+", "-", "*", "/", "%", "<", ">"];

fn lex(text: &str) -> Res<Vec<Tok>> {
    let cs: Vec<char> = text.chars().collect();
    let mut out: Vec<Tok> = Vec::new();
    let mut i: usize = 0;
    while i < cs.len() {
        let c: char = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_alphabetic() || c == '_' {
            let s: usize = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(cs[s..i].iter().collect()));
        } else if c.is_ascii_digit() {
            let s: usize = i;
            let mut dot: bool = false;
            while i < cs.len() && (cs[i].is_ascii_digit() || (cs[i] == '.' && !dot)) {
                if cs[i] == '.' {
                    dot = true;
                }
                i += 1;
            }
            let t: String = cs[s..i].iter().collect();
            if dot {
                out.push(Tok::Float(t.parse().map_err(|_| E::Format("rule: bad number"))?));
            } else {
                out.push(Tok::Int(t.parse().map_err(|_| E::Format("rule: bad number"))?));
            }
        } else if c == '\'' || c == '"' {
            let q: char = c;
            i += 1;
            let mut s: String = String::new();
            loop {
                if i >= cs.len() {
                    return Err(E::Format("rule: unterminated string"));
                }
                let d: char = cs[i];
                i += 1;
                if d == q {
                    break;
                }
                if d == '\\' && i < cs.len() {
                    s.push(cs[i]);
                    i += 1;
                } else {
                    s.push(d);
                }
            }
            out.push(Tok::Str(s));
        } else {
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            if let Some(s) = SYMS2.iter().find(|s| **s == two) {
                out.push(Tok::Sym(s));
                i += 2;
            } else if let Some(s) = SYMS1.iter().find(|s| s.starts_with(c)) {
                out.push(Tok::Sym(s));
                i += 1;
            } else {
                return Err(E::Msg(format!("rule: unexpected character {c}")));
            }
        }
    }
    out.push(Tok::End);
    Ok(out)
}

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn peek(&self) -> &Tok {
        &self.t[self.i]
    }

    fn bump(&mut self) -> Tok {
        let t: Tok = self.t[self.i].clone();
        if self.i + 1 < self.t.len() {
            self.i += 1;
        }
        t
    }

    fn sym(&mut self, s: &str) -> bool {
        if matches!(self.peek(), Tok::Sym(x) if *x == s) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn kw(&mut self, s: &str) -> bool {
        if matches!(self.peek(), Tok::Name(x) if x == s) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn need(&mut self, s: &str) -> Res<()> {
        if self.sym(s) {
            Ok(())
        } else {
            Err(E::Msg(format!("rule: expected {s}")))
        }
    }

    fn expr(&mut self, depth: usize) -> Res<Ast> {
        if depth > 120 {
            return Err(E::Format("rule: nesting too deep"));
        }
        let a: Ast = self.or_e(depth)?;
        if self.kw("if") {
            let c: Ast = self.or_e(depth)?;
            if !self.kw("else") {
                return Err(E::Format("rule: expected else"));
            }
            let b: Ast = self.expr(depth + 1)?;
            return Ok(Ast::If(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(a)
    }

    fn or_e(&mut self, depth: usize) -> Res<Ast> {
        let mut v: Vec<Ast> = vec![self.and_e(depth)?];
        while self.kw("or") {
            let x: Ast = self.and_e(depth)?;
            match x {
                Ast::Or(mut w) => v.append(&mut w),
                o => v.push(o),
            }
        }
        if v.len() == 1 {
            Ok(v.remove(0))
        } else {
            Ok(Ast::Or(v))
        }
    }

    fn and_e(&mut self, depth: usize) -> Res<Ast> {
        let mut v: Vec<Ast> = vec![self.not_e(depth)?];
        while self.kw("and") {
            let x: Ast = self.not_e(depth)?;
            match x {
                Ast::And(mut w) => v.append(&mut w),
                o => v.push(o),
            }
        }
        if v.len() == 1 {
            Ok(v.remove(0))
        } else {
            Ok(Ast::And(v))
        }
    }

    fn not_e(&mut self, depth: usize) -> Res<Ast> {
        if depth > 120 {
            return Err(E::Format("rule: nesting too deep"));
        }
        if self.kw("not") {
            let x: Ast = self.not_e(depth + 1)?;
            return Ok(Ast::Not(Box::new(x)));
        }
        self.cmp_e(depth)
    }

    fn cmp_op(&mut self) -> Option<Cmp> {
        let next_is = |p: &P, w: &str| matches!(p.t.get(p.i + 1), Some(Tok::Name(m)) if m == w);
        let (o, n): (Cmp, usize) = match self.peek() {
            Tok::Sym("==") => (Cmp::Eq, 1),
            Tok::Sym("!=") => (Cmp::Ne, 1),
            Tok::Sym("<") => (Cmp::Lt, 1),
            Tok::Sym("<=") => (Cmp::Le, 1),
            Tok::Sym(">") => (Cmp::Gt, 1),
            Tok::Sym(">=") => (Cmp::Ge, 1),
            Tok::Name(w) if w == "in" => (Cmp::In, 1),
            Tok::Name(w) if w == "is" => {
                if next_is(self, "not") {
                    (Cmp::Ne, 2)
                } else {
                    (Cmp::Eq, 1)
                }
            }
            Tok::Name(w) if w == "not" && next_is(self, "in") => (Cmp::NotIn, 2),
            _ => return None,
        };
        self.i += n;
        Some(o)
    }

    fn cmp_e(&mut self, depth: usize) -> Res<Ast> {
        let l: Ast = self.arith(depth)?;
        let mut rest: Vec<(Cmp, Ast)> = Vec::new();
        while let Some(o) = self.cmp_op() {
            let r: Ast = self.arith(depth)?;
            rest.push((o, r));
        }
        if rest.is_empty() {
            Ok(l)
        } else {
            Ok(Ast::Cmp(Box::new(l), rest))
        }
    }

    fn arith(&mut self, depth: usize) -> Res<Ast> {
        let mut l: Ast = self.term(depth)?;
        loop {
            let op: Bop = if self.sym("+") {
                Bop::Add
            } else if self.sym("-") {
                Bop::Sub
            } else {
                return Ok(l);
            };
            let r: Ast = self.term(depth)?;
            l = Ast::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn term(&mut self, depth: usize) -> Res<Ast> {
        let mut l: Ast = self.unary(depth)?;
        loop {
            let op: Bop = if self.sym("*") {
                Bop::Mul
            } else if self.sym("//") {
                Bop::Floor
            } else if self.sym("/") {
                Bop::Div
            } else if self.sym("%") {
                Bop::Rem
            } else {
                return Ok(l);
            };
            let r: Ast = self.unary(depth)?;
            l = Ast::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn unary(&mut self, depth: usize) -> Res<Ast> {
        if depth > 120 {
            return Err(E::Format("rule: nesting too deep"));
        }
        if self.sym("-") {
            let x: Ast = self.unary(depth + 1)?;
            return Ok(Ast::Neg(Box::new(x)));
        }
        self.post(depth)
    }

    fn args(&mut self, depth: usize) -> Res<Vec<Ast>> {
        let mut v: Vec<Ast> = Vec::new();
        if self.sym(")") {
            return Ok(v);
        }
        loop {
            v.push(self.expr(depth + 1)?);
            if self.sym(",") {
                if self.sym(")") {
                    return Ok(v);
                }
                continue;
            }
            self.need(")")?;
            return Ok(v);
        }
    }

    fn post(&mut self, depth: usize) -> Res<Ast> {
        let a: Ast = self.atom(depth)?;
        if let Ast::Name(n) = &a {
            if self.sym("(") {
                let v: Vec<Ast> = self.args(depth)?;
                return Ok(Ast::Call(n.clone(), v));
            }
            if self.sym("[") {
                let x: Ast = self.expr(depth + 1)?;
                self.need("]")?;
                return Ok(Ast::Index(n.clone(), Box::new(x)));
            }
        }
        Ok(a)
    }

    fn atom(&mut self, depth: usize) -> Res<Ast> {
        match self.bump() {
            Tok::Name(n) => match n.as_str() {
                "True" => Ok(Ast::Bool(true)),
                "False" => Ok(Ast::Bool(false)),
                "None" => Ok(Ast::Bool(false)),
                "and" | "or" | "not" | "in" | "is" | "if" | "else" => {
                    Err(E::Msg(format!("rule: unexpected keyword {n}")))
                }
                _ => Ok(Ast::Name(n)),
            },
            Tok::Str(s) => Ok(Ast::Str(s)),
            Tok::Int(n) => Ok(Ast::Int(n)),
            Tok::Float(f) => Ok(Ast::Float(f)),
            Tok::Sym("(") => {
                if self.sym(")") {
                    return Ok(Ast::Tuple(Vec::new()));
                }
                let first: Ast = self.expr(depth + 1)?;
                if self.sym(",") {
                    let mut v: Vec<Ast> = vec![first];
                    while !self.sym(")") {
                        v.push(self.expr(depth + 1)?);
                        if !self.sym(",") {
                            self.need(")")?;
                            break;
                        }
                    }
                    Ok(Ast::Tuple(v))
                } else {
                    self.need(")")?;
                    Ok(first)
                }
            }
            Tok::Sym("[") => {
                let mut v: Vec<Ast> = Vec::new();
                while !self.sym("]") {
                    v.push(self.expr(depth + 1)?);
                    if !self.sym(",") {
                        self.need("]")?;
                        break;
                    }
                }
                Ok(Ast::List(v))
            }
            Tok::Sym(s) => Err(E::Msg(format!("rule: unexpected {s}"))),
            Tok::End => Err(E::Format("rule: unexpected end")),
        }
    }
}

pub fn parse_rule(text: &str) -> Res<Ast> {
    let t: Vec<Tok> = lex(text)?;
    let mut p: P = P { t, i: 0 };
    let a: Ast = p.expr(0)?;
    if *p.peek() != Tok::End {
        return Err(E::Msg(format!("rule: trailing input in {text}")));
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> Ast {
        Ast::Name(s.to_string())
    }

    #[test]
    fn precedence_and_flattening() {
        let a: Ast = parse_rule("A or B and C or D").unwrap();
        assert_eq!(
            a,
            Ast::Or(vec![name("A"), Ast::And(vec![name("B"), name("C")]), name("D")])
        );
        let b: Ast = parse_rule("A and B and C").unwrap();
        assert_eq!(b, Ast::And(vec![name("A"), name("B"), name("C")]));
    }

    #[test]
    fn parens_do_not_flatten_across_groups_wrongly() {
        let a: Ast = parse_rule("(A or B) and C").unwrap();
        assert_eq!(a, Ast::And(vec![Ast::Or(vec![name("A"), name("B")]), name("C")]));
    }

    #[test]
    fn tuples_calls_and_strings() {
        let a: Ast = parse_rule("(Progressive_Hookshot, 2) and can_play(Song) and 'Gate Open'").unwrap();
        assert_eq!(
            a,
            Ast::And(vec![
                Ast::Tuple(vec![name("Progressive_Hookshot"), Ast::Int(2)]),
                Ast::Call("can_play".to_string(), vec![name("Song")]),
                Ast::Str("Gate Open".to_string()),
            ])
        );
    }

    #[test]
    fn comparisons_and_not() {
        let a: Ast = parse_rule("not (age == 'child') and x != 3 and y not in [1, 2]").unwrap();
        let Ast::And(v) = a else { panic!("expected and") };
        assert_eq!(v.len(), 3);
        assert!(matches!(&v[0], Ast::Not(_)));
        assert!(matches!(&v[1], Ast::Cmp(_, r) if r[0].0 == Cmp::Ne));
        assert!(matches!(&v[2], Ast::Cmp(_, r) if r[0].0 == Cmp::NotIn));
    }

    #[test]
    fn ternary_and_arithmetic() {
        let a: Ast = parse_rule("(1 + 2 * 3) if flag else 4.5").unwrap();
        let Ast::If(_, t, e) = a else { panic!("expected if") };
        assert_eq!(*e, Ast::Float(4.5));
        assert_eq!(
            *t,
            Ast::Bin(
                Bop::Add,
                Box::new(Ast::Int(1)),
                Box::new(Ast::Bin(Bop::Mul, Box::new(Ast::Int(2)), Box::new(Ast::Int(3))))
            )
        );
    }

    #[test]
    fn subscripts_and_multiline() {
        let a: Ast = parse_rule("skipped[Forest]\n   and\n True").unwrap();
        assert_eq!(
            a,
            Ast::And(vec![Ast::Index("skipped".to_string(), Box::new(name("Forest"))), Ast::Bool(true)])
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_rule("").is_err());
        assert!(parse_rule("A and").is_err());
        assert!(parse_rule("(A").is_err());
        assert!(parse_rule("A B").is_err());
        assert!(parse_rule("'open").is_err());
        assert!(parse_rule("A $ B").is_err());
    }

    #[test]
    fn deep_nesting_is_refused_not_crashed() {
        let t: String = format!("{}A{}", "(".repeat(5000), ")".repeat(5000));
        assert!(parse_rule(&t).is_err());
    }
}
