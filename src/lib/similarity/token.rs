#[cfg(test)]
use proptest::prelude::*;

pub trait Tokeniser<'a> {
    fn tokenise(content: &'a str) -> impl Iterator<Item = Token<'a>>;
}

pub struct SplitTokeniser {}
impl<'a> Tokeniser<'a> for SplitTokeniser {
    fn tokenise(content: &'a str) -> impl Iterator<Item = Token<'a>> {
        content
            .split(|c: char| !c.is_alphabetic())
            .filter(|s| !s.is_empty())
            .map(Token)
    }
}
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct Token<'a>(&'a str);

#[cfg(test)]
impl<'a> Arbitrary for Token<'a> {
    type Parameters = ();
    // The strategy will be to generate a String, leak it, and create a Token
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        let strategy = "[a-zA-Z]+".prop_map(|s| {
            let leaked_str: &'static str = Box::leak(s.into_boxed_str());
            Token(leaked_str)
        });

        strategy.boxed()
    }
}
