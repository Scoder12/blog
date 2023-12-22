use std::marker::PhantomData;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

// inspired by https://github.com/khonsulabs/pulldown-cmark-frontmatter/blob/main/src/lib.rs

enum State {
    Parsing,
    Done,
}

pub struct FrontmatterExtractor<'a, T>
where
    T: Iterator<Item = Event<'a>>,
{
    source: T,
    state: State,
    pub frontmatter: Option<Vec<CowStr<'a>>>,
}

impl<'a, T> FrontmatterExtractor<'a, T>
where
    T: Iterator<Item = Event<'a>>,
{
    pub fn new(parser: T) -> Self {
        Self {
            source: parser,
            state: State::Parsing,
            frontmatter: None,
        }
    }
}

impl<'a, T> Iterator for FrontmatterExtractor<'a, T>
where
    T: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Parsing => {}
            State::Done => {
                return self.source.next();
            }
        };

        macro_rules! bail {
            ($item:expr) => {
                self.state = State::Done;
                return $item;
            };
        }

        // ridiculously overengineered macro (lol)
        // We need to extract variables from the item by pattern matching
        // We also need to return the item if the guard fails
        // We can't return the original item variable, because the assignment partially
        //  moves out of it
        // So we have to rebuild the item by evalutating the pattern as an expression
        // If we bind a macro argument as a pattern (e.g. $pattern:pat), the parser
        //  will refuse to reparse it as an expression
        // Therefore, we need to take it as an unmodified token tree
        // The input tokens are in the format: <pattern tokens> (if <guard tokens>)?
        // In order to split on "if" without introducing parsing ambiguities, we have to
        //  recursively munch the input tokens one-by-one, accumulating the pattern until
        //  we receive an if.
        // In conclusion, this is so unnecessary complicated to avoid a couple lines of
        //  repeated code, but it's funny and I learned a lot making it so I'm keeping it :)
        macro_rules! match_next_helper {
            ([pattern: $($pattern:tt)*]) => {
                let item = self.source.next();
                let Some($($pattern)*) = item else {
                    bail!(item);
                };
            };
            ([pattern: $($pattern:tt)*] if $($tail:tt)*) => {
                match_next_helper!([pattern: $($pattern)*]);

                if !($($tail)*) {
                    bail!(Some($($pattern)*));
                }
            };
            ([pattern: $($pattern:tt)*] $tt:tt $($tail:tt)*) => {
                match_next_helper!([pattern: $($pattern)* $tt] $($tail)*)
            };
        }

        macro_rules! match_next {
            ($($input:tt)*) => {
                match_next_helper!([pattern: ] $($input)*)
            };
        }

        macro_rules! match_break {
            () => {
                match_next!(Event::SoftBreak | Event::HardBreak);
            };
        }

        const DELIMITER: &str = "+++";
        match_next!(Event::Start(Tag::Paragraph));
        match_next!(Event::Text(s) if s.as_ref() == DELIMITER);
        let mut lines = Vec::new();
        loop {
            match_break!();
            let item = self.source.next()?;
            let line = match item {
                Event::Text(l) => {
                    if l.as_ref() == DELIMITER {
                        break;
                    }
                    l
                }
                item => {
                    bail!(Some(item));
                }
            };
            lines.push(line);
        }
        match_next!(Event::End(Tag::Paragraph));

        self.frontmatter = Some(lines);
        self.state = State::Done;
        return self.source.next();
    }
}

mod tests {
    use pulldown_cmark::Parser;

    use super::FrontmatterExtractor;

    const OUTPUT_NONE: Option<Vec<String>> = None;

    fn testcase(input: impl AsRef<str>, output: Option<Vec<impl AsRef<str>>>) {
        let mut parser = FrontmatterExtractor::new(Parser::new(input.as_ref()));
        while let Some(_) = parser.next() {}
        let actual: Option<Vec<String>> = parser
            .frontmatter
            .map(|v| v.into_iter().map(|l| l.as_ref().to_owned()).collect());
        let expected: Option<Vec<String>> =
            output.map(|v| v.into_iter().map(|l| l.as_ref().to_owned()).collect());
        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_parse() {
        testcase(
            r#"+++
a
b
+++

abcd"#,
            Some(vec!["a", "b"]),
        );
    }

    #[test]
    fn no_para_end() {
        testcase(
            r#"+++
a
b
+++
abcd"#,
            OUTPUT_NONE,
        );
    }
}
