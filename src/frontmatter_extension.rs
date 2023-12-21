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
    phantomdata: PhantomData<&'a T>,
}

impl<'a, T> FrontmatterExtractor<'a, T>
where
    T: Iterator<Item = Event<'a>>,
{
    pub fn new(parser: T) -> Self {
        Self {
            source: parser,
            state: State::Parsing,
            phantomdata: PhantomData,
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

        macro_rules! as_expr {
            ($e:expr) => {
                $e
            };
        }
        macro_rules! as_pat {
            ($p:pat) => {
                $p
            };
        }

        macro_rules! bail {
            ($item:expr) => {
                self.state = State::Done;
                return $item;
            };
        }

        /*macro_rules! match_next {
            ($pattern:pat $(if $guard:expr)? $(,)?) => {
                let item = self.source.next();
                let Some($pattern) = item else {
                    self.state = State::Done;
                    return item;
                };
                {
                    let g = guh!($pattern);
                    $(if !$guard {
                        bail!(g);
                    };)?
                }
            };
        }*/
        macro_rules! match_next {
            ([pattern: $($pattern:tt)* $(if $guard:expr)? $(,)?) => {
                let item = self.source.next();
                let Some(as_pat!($($pattern)*)) = item else {
                    self.state = State::Done;
                    return item;
                };
                {
                    let g = as_expr!($($pattern)*);
                    $(if !$guard {
                        bail!(g);
                    };)?
                }
            };
            ($($input:tt)*) => {
                match_next!([pattern: ] $($input)*)
            }
        }

        match_next!(Event::Start(Tag::Paragraph));
        trace_macros!(true);
        match_next!(Event::Text(s) if s.as_ref() == "+++");
        trace_macros!(false);
        let a = Event::Text(s);
        println!("{:#?}", s);

        self.state = State::Done;
        return self.source.next();
    }
}
