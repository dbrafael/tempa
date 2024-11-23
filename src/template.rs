use std::collections::VecDeque;

pub type ReplacementCount = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    String(&'a str),
    Replacement(&'a str),
}

#[derive(Clone, Debug)]
pub struct Template<'a> {
    tokens: VecDeque<Token<'a>>,
    open: &'a str,
    close: &'a str,
}

impl<'a> Template<'a> {
    pub fn from_str(s: &'a str, open: &'a str, close: &'a str) -> Self {
        Self {
            open,
            close,
            tokens: string_to_toks(s, open, close).into(),
        }
    }

    pub fn apply(&self, replacement_map: &yaml_rust::Yaml) -> (ReplacementCount, String) {
        let mut ret = String::new();
        let mut count = 0;

        for tok in self.clone() {
            match tok {
                Token::String(s) => ret.push_str(s),
                Token::Replacement(s) => {
                    let parts = s.split(".");
                    let mut map = replacement_map;

                    for part in parts {
                        map = &map[part];
                    }

                    let replacement = match map.as_str() {
                        Some(s) => s,
                        None => &format!("{}{s}{}", self.open, self.close),
                    };

                    ret.push_str(replacement);
                    count += 1;
                }
            }
        }

        (count, ret)
    }
}

impl<'a> Iterator for Template<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.pop_front()
    }
}

fn string_to_toks<'a>(mut s: &'a str, open: &str, close: &str) -> Vec<Token<'a>> {
    let mut ret = Vec::new();
    let mut inside = false;

    while s.len() > 0 {
        let next_delim = s.find(match inside {
            true => close,
            false => open,
        });

        match next_delim {
            Some(idx) => {
                let before_token = &s[0..idx];
                let after_token = &s[idx + open.len()..];

                if before_token.len() > 0 {
                    ret.push(match inside {
                        true => Token::Replacement(before_token),
                        false => Token::String(before_token),
                    });
                }

                inside = !inside;
                s = after_token;
            }
            None => {
                ret.push(Token::String(&s[0..]));
                break;
            }
        }
    }

    ret
}

#[test]
fn str_to_template() {
    let text = "%%name%% Hello I am the %%name2%%, pleased to meet you";
    let template = Template::from_str(text, "%%", "%%");

    assert_eq!(template.tokens.iter().count(), 4);
    assert_eq!(template.tokens[0], Token::Replacement("name"));
    assert_eq!(template.tokens[1], Token::String(" Hello I am the "));
    assert_eq!(template.tokens[2], Token::Replacement("name2"));
    assert_eq!(template.tokens[3], Token::String(", pleased to meet you"));
}

#[test]
fn different_delims() {
    let text = "xx0%nameabc% Hello I am the %%name2%%, pleased to meet you";
    let template = Template::from_str(text, "xx0%", "abc%");

    assert_eq!(template.tokens.iter().count(), 2);
    assert_eq!(template.tokens[0], Token::Replacement("name"));
    assert_eq!(
        template.tokens[1],
        Token::String(" Hello I am the %%name2%%, pleased to meet you")
    );

    assert_eq!(template.data, text);
}

#[test]
fn apply_template() {
    let text =
        "%%names.name%% Hello I am the %%names.name2%%, pleased to meet you %%names.invalid%%";
    let template = Template::from_str(text, "%%", "%%");
    let yaml =
        yaml_rust::YamlLoader::load_from_str("names:\n  name: Test Name\n  name2: Test Name 2")
            .unwrap()[0]
            .clone();
    let replaced = template.apply(&yaml);

    assert_eq!(
        replaced,
        "Test Name Hello I am the Test Name 2, pleased to meet you %%names.invalid%%"
    );
}
