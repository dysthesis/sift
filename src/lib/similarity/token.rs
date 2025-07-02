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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn tokens_are_always_alphabetic(
            content in any::<String>()
        ) {
            let tokens = SplitTokeniser::tokenise(&content);

            for token in tokens {
                for c in token.0.chars() {
                    prop_assert!(
                        c.is_alphabetic(),
                        "Token '{:?}' contained non-alphabetic char '{}' from original content: '{}'",
                        token, c, content
                    );
                }
            }
        }
    }

    proptest! {
        #[test]
        fn tokens_are_never_empty(content in any::<String>()) {
            let tokens = SplitTokeniser::tokenise(&content);

            for token in tokens {
                prop_assert!(
                    !token.0.is_empty(),
                    "Tokeniser produced an empty token from content: '{}'",
                    content
                );
            }
        }
    }

    proptest! {
        #[test]
        fn tokenising_conserves_all_alphabetic_characters(content in any::<String>()) {
            let expected_chars: String = content.chars().filter(|c| c.is_alphabetic()).collect();

            let actual_chars: String = SplitTokeniser::tokenise(&content)
                .map(|token| token.0)
                .collect();

            prop_assert_eq!(
                actual_chars,
                expected_chars,
                "Tokenisation did not conserve alphabetic characters for content: '{}'",
                content
            );
        }
    }

    #[test]
    fn simple_tokenisation_example() {
        let content = "  Hello, world! 123 This is Rust.  ";
        let tokens: Vec<Token> = SplitTokeniser::tokenise(content).collect();

        assert_eq!(
            tokens,
            vec![
                Token("Hello"),
                Token("world"),
                Token("This"),
                Token("is"),
                Token("Rust")
            ]
        );
    }

    #[test]
    fn tokenisation_of_empty_string() {
        let content = "";
        let tokens: Vec<Token> = SplitTokeniser::tokenise(content).collect();
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenisation_of_non_alphabetic_string() {
        let content = "123 !@#$%^&*()_+";
        let tokens: Vec<Token> = SplitTokeniser::tokenise(content).collect();
        assert!(tokens.is_empty());
    }
}
