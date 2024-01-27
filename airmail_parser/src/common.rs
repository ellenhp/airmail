use nom::{bytes::complete::take_while, IResult};

pub fn query_sep(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_whitespace() || c.is_ascii_punctuation())(input)
}

pub fn query_term(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| !c.is_whitespace() && !c.is_ascii_punctuation())(input)
}
