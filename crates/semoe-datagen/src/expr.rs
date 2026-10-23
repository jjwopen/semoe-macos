//! 날개셋 `KeyTable` 조건식을 순서 있는 규칙 목록(`KeyRule`)으로 푼다.
//!
//! 범용 해석기를 만들지 않는다(계획서 4.1). 세모이 XML에 실제로 쓰이는 꼴만 받아들이고,
//! 그 밖의 식은 오류로 처리해 변환 누락을 막는다.
//!
//! 받아들이는 문법:
//! ```text
//! expr    := ternary
//! ternary := cond '?' value ':' ternary | value
//! value   := '(' 'T' '=' NUM ',' leaf ')' | leaf | ternary
//! cond    := atom ('&&' atom)*
//! atom    := 'T' '==' NUM | 'P' | 'N' '&' '2' | '!'? ('E' | 'F')
//! leaf    := NUM | 'H' d '|' (NAME | NUM) | 'C' d '|' NUM
//! ```

use semoe_core::data::{Cond, KeyAction, KeyRule, Unit};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Num(u32),
    Ident(String),
    /// `H<d>|이름` 또는 `H<d>|숫자`
    Unit(String),
    /// `C<group>|<n>`
    Cmd(u32, u32),
    Op(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let num = |s: &str| -> Result<u32, String> {
        match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            Some(h) => u32::from_str_radix(h, 16),
            None => s.parse(),
        }
        .map_err(|e| format!("숫자 {s:?}: {e}"))
    };
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let rest = &src[i..];
        if let Some(op) = [
            "==", "&&", "<=", ">=", "!=", "?", ":", "!", "&", "(", ")", ",", "=", "<", ">", "|",
        ]
        .into_iter()
        .find(|op| rest.starts_with(op))
        {
            out.push(Tok::Op(op));
            i += op.len();
            continue;
        }
        let end = rest
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(format!("알 수 없는 문자 {c:?}"));
        }
        let word = &rest[..end];
        i += end;
        // H3|이름, C1|n: 접두어 한 글자 + 숫자 다음에 '|'가 붙은 꼴은 한 토큰으로 묶는다.
        let tagged = word.len() == 2
            && matches!(&word[..1], "H" | "C")
            && word.as_bytes()[1].is_ascii_digit()
            && src[i..].starts_with('|');
        if tagged {
            let arg_start = i + 1;
            let arg_len = src[arg_start..]
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(src.len() - arg_start);
            let arg = &src[arg_start..arg_start + arg_len];
            i = arg_start + arg_len;
            out.push(if word.starts_with('H') {
                Tok::Unit(arg.to_string())
            } else {
                Tok::Cmd(num(&word[1..])?, num(arg)?)
            });
        } else if word.as_bytes()[0].is_ascii_digit() {
            out.push(Tok::Num(num(word)?));
        } else {
            out.push(Tok::Ident(word.to_string()));
        }
    }
    Ok(out)
}

struct Parser<'a, F> {
    toks: Vec<Tok>,
    pos: usize,
    resolve: &'a F,
}

impl<F: Fn(&str) -> Result<Unit, String>> Parser<'_, F> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Result<Tok, String> {
        let t = self.toks.get(self.pos).cloned().ok_or("식이 일찍 끝남")?;
        self.pos += 1;
        Ok(t)
    }

    fn expect(&mut self, op: &'static str) -> Result<(), String> {
        match self.next()? {
            Tok::Op(o) if o == op => Ok(()),
            t => Err(format!("{op:?}가 와야 하는데 {t:?}")),
        }
    }

    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(o)) if *o == op)
    }

    /// 지금 위치가 조건(`?` 앞)인지 미리 본다: 같은 깊이에서 `?`가 `:`보다 먼저 나오면 조건이다.
    fn at_condition(&self) -> bool {
        let mut depth = 0;
        for t in &self.toks[self.pos..] {
            match t {
                Tok::Op("(") => depth += 1,
                Tok::Op(")") if depth == 0 => return false,
                Tok::Op(")") => depth -= 1,
                Tok::Op("?") if depth == 0 => return true,
                Tok::Op(":") | Tok::Op(",") if depth == 0 => return false,
                _ => {}
            }
        }
        false
    }

    /// ternary를 읽어 규칙 목록으로 펼친다. `conds`는 바깥 조건들이다.
    ///
    /// `c ? a : b`는 "c가 맞으면 a의 규칙들, 아니면 b의 규칙들"이다. 규칙을 위에서부터 첫 일치로
    /// 고르므로, a의 규칙에는 c를 붙이고 b의 규칙에는 ¬c를 붙이지 않아도 된다(a의 마지막 규칙이
    /// c만으로 항상 맞으므로 c인 입력은 b까지 내려가지 않는다).
    fn ternary(&mut self, conds: &[Cond], out: &mut Vec<KeyRule>) -> Result<(), String> {
        if self.at_condition() {
            let mut inner = conds.to_vec();
            inner.extend(self.cond()?);
            self.expect("?")?;
            self.ternary(&inner, out)?;
            self.expect(":")?;
            self.ternary(conds, out)
        } else {
            let rule = self.value(conds)?;
            out.push(rule);
            Ok(())
        }
    }

    fn cond(&mut self) -> Result<Vec<Cond>, String> {
        let mut cs = vec![self.atom()?];
        while self.is_op("&&") {
            self.next()?;
            cs.push(self.atom()?);
        }
        Ok(cs)
    }

    fn atom(&mut self) -> Result<Cond, String> {
        let negated = self.is_op("!");
        if negated {
            self.next()?;
        }
        let t = self.next()?;
        let cond = match &t {
            Tok::Ident(v) if v == "T" => {
                self.expect("==")?;
                match self.next()? {
                    Tok::Num(n) => Cond::Stage { is: n },
                    t => return Err(format!("T== 뒤에 숫자가 아님: {t:?}")),
                }
            }
            Tok::Ident(v) if v == "P" => Cond::CapsLock,
            Tok::Ident(v) if v == "N" => {
                self.expect("&")?;
                match self.next()? {
                    Tok::Num(2) => Cond::ScrollLock,
                    t => return Err(format!("N& 뒤에 2가 아님: {t:?}")),
                }
            }
            Tok::Ident(v) if v == "E" || v == "F" => {
                return Ok(Cond::Flag {
                    name: v.chars().next().unwrap(),
                    value: !negated,
                });
            }
            t => return Err(format!("알 수 없는 조건 {t:?}")),
        };
        if negated {
            return Err(format!("부정을 지원하지 않는 조건: !{cond:?}"));
        }
        Ok(cond)
    }

    fn value(&mut self, conds: &[Cond]) -> Result<KeyRule, String> {
        let mut set_stage = None;
        let paren = self.is_op("(");
        if paren {
            self.next()?;
            match (self.next()?, self.next()?, self.next()?) {
                (Tok::Ident(v), Tok::Op("="), Tok::Num(n)) if v == "T" => set_stage = Some(n),
                t => return Err(format!("(T=n, …) 꼴이 아님: {t:?}")),
            }
            self.expect(",")?;
        }
        let action = match self.next()? {
            Tok::Num(0) => KeyAction::Nothing,
            Tok::Num(n) => KeyAction::Char {
                ch: char::from_u32(n).ok_or(format!("문자가 아닌 값 {n:#x}"))?,
            },
            Tok::Unit(name) => KeyAction::Unit {
                unit: (self.resolve)(&name)?,
            },
            Tok::Cmd(group, n) => KeyAction::Command { group, n },
            t => return Err(format!("값이 와야 하는데 {t:?}")),
        };
        if paren {
            self.expect(")")?;
        }
        Ok(KeyRule {
            when: conds.to_vec(),
            action,
            set_stage,
        })
    }
}

/// 조건식을 규칙 목록으로 바꾼다. `resolve`는 `H<d>|` 뒤의 낱자 이름(또는 숫자)을 낱자로 바꾼다.
pub fn key_rules(
    src: &str,
    resolve: &impl Fn(&str) -> Result<Unit, String>,
) -> Result<Vec<KeyRule>, String> {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
        resolve,
    };
    let mut out = Vec::new();
    p.ternary(&[], &mut out)
        .map_err(|e| format!("{e} — 식: {src}"))?;
    if p.pos != p.toks.len() {
        return Err(format!(
            "해석하지 못한 꼬리 {:?} — 식: {src}",
            &p.toks[p.pos..]
        ));
    }
    // 같은 조건이 앞에 이미 있으면 뒤 규칙은 영영 쓰이지 않는다(원본 식의 중복). 지운다.
    let mut seen: Vec<Vec<Cond>> = Vec::new();
    out.retain(|r| {
        let dup = seen.contains(&r.when);
        seen.push(r.when.clone());
        !dup
    });
    match out.last() {
        Some(r) if r.when.is_empty() => Ok(out),
        _ => Err(format!("조건 없는 마지막 규칙이 없음 — 식: {src}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semoe_core::data::Slot;

    fn resolve(name: &str) -> Result<Unit, String> {
        match name {
            "_Q" => Ok(Unit {
                slot: Slot::Jong,
                ch: 'ᆼ',
            }),
            "G_" => Ok(Unit {
                slot: Slot::Cho,
                ch: 'ᄀ',
            }),
            _ => Err(format!("모르는 낱자 {name}")),
        }
    }

    fn ch(c: char) -> KeyAction {
        KeyAction::Char { ch: c }
    }

    #[test]
    fn flattens_nested_ternaries_in_order() {
        let rules = key_rules(
            "T==101 ? 0x25C7 : T==102 ? 0x25C8 : P ? N&2 ? 0x21E6 : 0x2190 : H3|_Q",
            &resolve,
        )
        .unwrap();
        let got: Vec<_> = rules
            .iter()
            .map(|r| (r.when.clone(), r.action.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (vec![Cond::Stage { is: 101 }], ch('◇')),
                (vec![Cond::Stage { is: 102 }], ch('◈')),
                (vec![Cond::CapsLock, Cond::ScrollLock], ch('⇦')),
                (vec![Cond::CapsLock], ch('←')),
                (
                    vec![],
                    KeyAction::Unit {
                        unit: resolve("_Q").unwrap()
                    }
                ),
            ]
        );
    }

    #[test]
    fn conjunction_negated_flags_command_and_assignment() {
        let rules = key_rules(
            "T==101 ? (T=0x65, 0x25CB) : P ? T==100&&!E&&!F ? C1|101 : 0 : H3|G_",
            &resolve,
        )
        .unwrap();
        assert_eq!(rules[0].set_stage, Some(101));
        assert_eq!(rules[0].action, ch('○'));
        assert_eq!(
            rules[1].when,
            vec![
                Cond::CapsLock,
                Cond::Stage { is: 100 },
                Cond::Flag {
                    name: 'E',
                    value: false
                },
                Cond::Flag {
                    name: 'F',
                    value: false
                },
            ]
        );
        assert_eq!(rules[1].action, KeyAction::Command { group: 1, n: 101 });
        assert_eq!(rules[2].action, KeyAction::Nothing);
    }

    #[test]
    fn rejects_unknown_constructs() {
        assert!(key_rules("X ? 1 : 2", &resolve).is_err());
        assert!(key_rules("T<100 ? 1 : 2", &resolve).is_err());
        assert!(key_rules("H3|nope", &resolve).is_err());
        assert!(key_rules("1 2", &resolve).is_err());
    }
}
