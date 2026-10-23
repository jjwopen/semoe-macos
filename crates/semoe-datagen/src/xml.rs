//! XML을 단순한 요소 트리로 읽는다 (날개셋 설정 XML, xlsx 안의 XML).

use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Element>,
    /// 바로 아래 텍스트(자식 요소의 텍스트는 포함하지 않음)
    pub text: String,
}

impl Element {
    /// 이름이 `name`인 첫 자손(깊이 우선).
    pub fn find(&self, name: &str) -> Option<&Element> {
        self.children.iter().find_map(|c| {
            if c.name == name {
                Some(c)
            } else {
                c.find(name)
            }
        })
    }

    /// 이름이 `name`인 자식들.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// 자신과 모든 자손의 텍스트를 이어 붙인다.
    pub fn all_text(&self) -> String {
        let mut s = self.text.clone();
        for c in &self.children {
            s += &c.all_text();
        }
        s
    }

    pub fn attr(&self, key: &str) -> Result<&str, String> {
        self.attrs
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("<{}>에 {key} 속성이 없음", self.name))
    }
}

pub fn parse(text: &str) -> Result<Element, String> {
    let mut reader = Reader::from_str(text);
    let mut stack = vec![Element {
        name: "#document".into(),
        ..Default::default()
    }];
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("XML 오류 (위치 {}): {e}", reader.buffer_position()))?
        {
            Event::Start(e) => stack.push(element(&e)?),
            Event::Empty(e) => {
                let el = element(&e)?;
                stack.last_mut().unwrap().children.push(el);
            }
            Event::End(_) => {
                let el = stack.pop().unwrap();
                stack
                    .last_mut()
                    .ok_or("닫는 태그가 남음")?
                    .children
                    .push(el);
            }
            Event::Text(t) => {
                let t = t.decode().map_err(|e| format!("텍스트 오류: {e}"))?;
                let t = quick_xml::escape::unescape(&t).map_err(|e| format!("텍스트 오류: {e}"))?;
                stack.last_mut().unwrap().text += &t;
            }
            Event::GeneralRef(r) => {
                // &amp; 같은 참조는 따로 온다.
                let name = r.decode().map_err(|e| format!("참조 오류: {e}"))?;
                let entity = format!("&{name};");
                let s =
                    quick_xml::escape::unescape(&entity).map_err(|e| format!("참조 오류: {e}"))?;
                stack.last_mut().unwrap().text += &s;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    match <[Element; 1]>::try_from(stack) {
        Ok([doc]) => Ok(doc),
        Err(_) => Err("닫히지 않은 태그가 있음".into()),
    }
}

fn element(e: &BytesStart) -> Result<Element, String> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut attrs = BTreeMap::new();
    for a in e.attributes() {
        let a = a.map_err(|err| format!("<{name}> 속성 오류: {err}"))?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let value = a
            .unescape_value()
            .map_err(|err| format!("<{name} {key}> 값 오류: {err}"))?;
        attrs.insert(key, value.into_owned());
    }
    Ok(Element {
        name,
        attrs,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_empty_elements_with_entities() {
        let doc = parse(r#"<a x="1 &lt; 2"><b/><c y="&#x1100;"><d/></c></a>"#).unwrap();
        let a = doc.find("a").unwrap();
        assert_eq!(a.attr("x").unwrap(), "1 < 2");
        assert_eq!(a.find("c").unwrap().attr("y").unwrap(), "ᄀ");
        assert!(a.find("d").is_some());
        assert_eq!(a.children_named("b").count(), 1);
        let t = parse("<si><r><t>가&amp;</t></r><r><t>나</t></r></si>").unwrap();
        assert_eq!(t.find("si").unwrap().all_text(), "가&나");
    }
}
