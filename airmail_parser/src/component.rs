use std::{fmt::Formatter, sync::Arc};

use crate::{
    common::{query_sep, query_term},
    fst::parse_fst,
};
use airmail_common::{
    dicts::*,
    fst::{search_fst, FstMatchMode},
};
use nom::{bytes::complete::take_while, IResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryComponentType {
    CategoryComponent,
    NearComponent,
    HouseNumberComponent,
    RoadComponent,
    IntersectionComponent,
    SublocalityComponent,
    LocalityComponent,
    RegionComponent,
    CountryComponent,
    PlaceNameComponent,
    IntersectionJoinWordComponent,
}

pub trait TriviallyConstructibleComponent: QueryComponent {
    fn new(text: String) -> Self;
}

pub trait QueryComponent {
    fn text(&self) -> &str;

    fn penalty_mult(&self) -> f32;

    fn debug_name(&self) -> &'static str;

    fn component_type(&self) -> QueryComponentType;

    fn subcomponents(&self) -> Vec<Arc<dyn QueryComponent>> {
        Vec::new()
    }
}

impl std::fmt::Debug for dyn QueryComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let subcomponents = self.subcomponents();
        if subcomponents.is_empty() {
            return f
                .debug_struct(self.debug_name())
                .field("text", &self.text())
                .field("penalty_mult", &self.penalty_mult())
                .finish();
        } else {
            let mut formatter = f.debug_struct(self.debug_name());
            for (i, subcomponent) in subcomponents.iter().enumerate() {
                formatter.field(&format!("subcomponent_{}", i), subcomponent);
            }
            formatter
                .field("text", &self.text())
                .field("penalty_mult", &self.penalty_mult())
                .finish()
        }
    }
}

fn parse_component<C: TriviallyConstructibleComponent>(
    text: &str,
    parser: fn(&str) -> IResult<&str, &str>,
) -> Vec<(C, &str)> {
    let mut scenarios = Vec::new();
    let mut sublist_len = 0;
    let mut sep_len = 0;

    let max_sublist_len = if let Ok((_, token)) = parser(text) {
        token.len()
    } else {
        return scenarios;
    };

    loop {
        if sublist_len + sep_len > max_sublist_len {
            break;
        }
        if let Ok((remainder, next_subtoken)) = query_term(&text[sublist_len + sep_len..]) {
            if next_subtoken.is_empty() {
                break;
            }
            sublist_len += next_subtoken.len();
            if let Ok((_, token)) = parser(&text[..sublist_len + sep_len]) {
                if token.len() == sublist_len + sep_len {
                    let component = C::new(token.to_string());
                    scenarios.push((component, &text[sublist_len + sep_len..]));
                }
            }
            // Accumulate the old separator length, then look for a new one.
            sublist_len += sep_len;
            if let Ok((_, sep)) = query_sep(remainder) {
                sep_len = sep.len();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    return scenarios;
}

macro_rules! define_component {
    ($name:ident, $parser:ident, $penalty_lambda:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            text: String,
        }

        impl TriviallyConstructibleComponent for $name {
            fn new(text: String) -> Self {
                Self { text }
            }
        }

        impl $name {
            pub fn parse(text: &str) -> Vec<(Self, &str)> {
                parse_component::<Self>(text, $parser)
            }

            fn parse_boxed(text: &str) -> Vec<(Arc<dyn QueryComponent>, &str)> {
                parse_component::<Self>(text, $parser)
                    .into_iter()
                    .map(|(component, remainder)| {
                        (Arc::new(component) as Arc<dyn QueryComponent>, remainder)
                    })
                    .collect()
            }
        }

        impl QueryComponent for $name {
            fn text(&self) -> &str {
                &self.text
            }
            fn debug_name(&self) -> &'static str {
                stringify!($name)
            }
            fn component_type(&self) -> QueryComponentType {
                QueryComponentType::$name
            }
            fn penalty_mult(&self) -> f32 {
                ($penalty_lambda)(&self.text)
            }
        }
    };
}

fn parse_category(text: &str) -> IResult<&str, &str> {
    parse_fst(
        &category_words_fst(),
        FstMatchMode::GreedyLevenshtein(0),
        text,
    )
}

define_component!(CategoryComponent, parse_category, |_| 1.0f32);

fn parse_near(text: &str) -> IResult<&str, &str> {
    parse_fst(
        &nearby_words_fst(),
        FstMatchMode::GreedyLevenshtein(0),
        text,
    )
}

define_component!(NearComponent, parse_near, |text: &str| 1.5f32
    .powi(text.split_whitespace().count() as i32));

fn parse_intersection_join_word(text: &str) -> IResult<&str, &str> {
    parse_fst(
        &intersection_join_words_fst(),
        FstMatchMode::GreedyLevenshtein(0),
        text,
    )
}

define_component!(
    IntersectionJoinWordComponent,
    parse_intersection_join_word,
    |_| 1.0f32
);

fn parse_house_number(text: &str) -> IResult<&str, &str> {
    // TODO: This should be more general. Not all house numbers are numbers.
    take_while(|c: char| c.is_ascii_digit())(text)
}

define_component!(HouseNumberComponent, parse_house_number, |_| 1.0f32);

#[derive(Debug, Clone)]
pub struct RoadComponent {
    text: String,
    penalty_mult: f32,
}

impl RoadComponent {
    // This is the base penalty for a token missing a street suffix.
    const PENALTY_MISSING_STREET_SUFFIX: f32 = 0.5f32;
    // This is a decay value for each additional token missing a street suffix. Total penalty is `base * decay ^ (num_tokens)`.
    const PENALTY_MISSING_STREET_SUFFIX_DECAY: f32 = 0.8f32;

    fn new(text: String, penalty_mult: f32) -> Self {
        Self { text, penalty_mult }
    }

    fn parse(text: &str) -> Vec<(Self, &str)> {
        // These scenarios are all going to be penalized for missing a street suffix.
        let mut scenarios = Vec::new();
        let mut substring_len = if let Ok((_, token)) = query_term(text) {
            token.len()
        } else {
            return scenarios;
        };

        scenarios.push((
            Self::new(
                text[..substring_len].to_string(),
                Self::PENALTY_MISSING_STREET_SUFFIX,
            ),
            &text[substring_len..],
        ));

        let mut sep_len = if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
            sep.len()
        } else {
            return scenarios;
        };

        for i in 1..3 {
            if let Ok((remainder, next_token)) = parse_fst(
                &street_suffixes_fst(),
                FstMatchMode::GreedyLevenshtein(0),
                &text[substring_len + sep_len..],
            ) {
                // Don't even bother returning penalized scenarios because suffixes make things very unambiguous.
                let component = Self::new(
                    text[..substring_len + sep_len + next_token.len()].to_string(),
                    1.2f32,
                );
                return vec![(component, remainder)];
            }
            // If we couldn't find a suffix, parse the next token and accumluate it then penalize this scenario.
            substring_len += if let Ok((_, token)) = query_term(&text[substring_len + sep_len..]) {
                if token.is_empty() {
                    break;
                }
                token.len()
            } else {
                break;
            };
            substring_len += sep_len;
            scenarios.push((
                Self::new(
                    text[..substring_len].to_string(),
                    Self::PENALTY_MISSING_STREET_SUFFIX
                        * Self::PENALTY_MISSING_STREET_SUFFIX_DECAY.powi(i),
                ),
                &text[substring_len..],
            ));
            if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
                sep_len = sep.len();
            } else {
                break;
            }
        }
        return scenarios;
    }

    pub fn parse_boxed(text: &str) -> Vec<(Arc<dyn QueryComponent>, &str)> {
        Self::parse(text)
            .into_iter()
            .map(|(component, remainder)| {
                (Arc::new(component) as Arc<dyn QueryComponent>, remainder)
            })
            .collect()
    }
}

impl QueryComponent for RoadComponent {
    fn text(&self) -> &str {
        &self.text
    }

    fn penalty_mult(&self) -> f32 {
        self.penalty_mult
    }

    fn debug_name(&self) -> &'static str {
        "RoadComponent"
    }

    fn component_type(&self) -> QueryComponentType {
        QueryComponentType::RoadComponent
    }
}

fn parse_sublocality(text: &str) -> IResult<&str, &str> {
    parse_fst(&sublocality_fst(), FstMatchMode::GreedyLevenshtein(0), text)
}

define_component!(SublocalityComponent, parse_sublocality, |_| 0.9f32);

#[derive(Debug, Clone)]
pub struct LocalityComponent {
    text: String,
}

impl LocalityComponent {
    fn new(text: String) -> Self {
        Self { text }
    }

    pub fn parse(text: &str) -> Vec<(Self, &str)> {
        let mut scenarios = Vec::new();
        let mut substring_len = if let Ok((_, token)) = query_term(text) {
            token.len()
        } else {
            return scenarios;
        };

        scenarios.push((
            Self::new(text[..substring_len].to_string()),
            &text[substring_len..],
        ));

        let mut sep_len = if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
            sep.len()
        } else {
            return scenarios;
        };

        loop {
            substring_len += if let Ok((_, token)) = query_term(&text[substring_len + sep_len..]) {
                if token.is_empty() {
                    break;
                }
                token.len()
            } else {
                break;
            };
            substring_len += sep_len;
            scenarios.push((
                Self::new(text[..substring_len].to_string()),
                &text[substring_len..],
            ));
            if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
                sep_len = sep.len();
            } else {
                break;
            }
        }
        return scenarios;
    }

    pub fn parse_boxed(text: &str) -> Vec<(Arc<dyn QueryComponent>, &str)> {
        Self::parse(text)
            .into_iter()
            .map(|(component, remainder)| {
                (Arc::new(component) as Arc<dyn QueryComponent>, remainder)
            })
            .collect()
    }
}

impl QueryComponent for LocalityComponent {
    fn text(&self) -> &str {
        &self.text
    }

    fn penalty_mult(&self) -> f32 {
        if search_fst(localities_fst(), self.text.clone(), 0, false) {
            1.1f32
        } else {
            0.5f32
        }
    }

    fn debug_name(&self) -> &'static str {
        "LocalityComponent"
    }

    fn component_type(&self) -> QueryComponentType {
        QueryComponentType::LocalityComponent
    }
}

fn parse_region(text: &str) -> IResult<&str, &str> {
    parse_fst(&regions_fst(), FstMatchMode::GreedyLevenshtein(0), text)
}

define_component!(RegionComponent, parse_region, |_| 1.0f32);

fn parse_country(text: &str) -> IResult<&str, &str> {
    parse_fst(&countries_fst(), FstMatchMode::GreedyLevenshtein(0), text)
}

define_component!(CountryComponent, parse_country, |_| 1.0f32);

#[derive(Debug, Clone)]
pub struct IntersectionComponent {
    text: String,
    road1: RoadComponent,
    intersection_join_word: IntersectionJoinWordComponent,
    road2: RoadComponent,
}

impl IntersectionComponent {
    fn new(
        text: String,
        road1: RoadComponent,
        intersection_join_word: IntersectionJoinWordComponent,
        road2: RoadComponent,
    ) -> Self {
        Self {
            text,
            road1,
            intersection_join_word,
            road2,
        }
    }

    pub fn road1(&self) -> &RoadComponent {
        &self.road1
    }

    pub fn road2(&self) -> &RoadComponent {
        &self.road2
    }

    pub fn intersection_join_word(&self) -> &IntersectionJoinWordComponent {
        &self.intersection_join_word
    }

    pub fn parse(text: &str) -> Vec<(Self, &str)> {
        let mut scenarios = Vec::new();
        let road1_scenarios = RoadComponent::parse(text);
        for (road1, remainder) in road1_scenarios {
            let (remainder, first_sep) = if let Ok((remainder, first_sep)) = query_sep(remainder) {
                (remainder, first_sep)
            } else {
                (remainder, "")
            };
            let intersection_join_word_scenarios = IntersectionJoinWordComponent::parse(remainder);
            for (intersection_join_word, remainder) in intersection_join_word_scenarios {
                let (remainder, second_sep) =
                    if let Ok((remainder, second_sep)) = query_sep(remainder) {
                        (remainder, second_sep)
                    } else {
                        (remainder, "")
                    };

                let road2_scenarios = RoadComponent::parse(remainder);
                for (road2, remainder) in road2_scenarios {
                    let remainder = remainder.trim_start();
                    let component = Self::new(
                        text[..road1.text().len()
                            + first_sep.len()
                            + intersection_join_word.text().len()
                            + second_sep.len()
                            + road2.text().len()]
                            .to_string(),
                        road1.clone(),
                        intersection_join_word.clone(),
                        road2.clone(),
                    );
                    scenarios.push((component, remainder));
                }
            }
        }
        scenarios
    }

    pub fn parse_boxed(text: &str) -> Vec<(Arc<dyn QueryComponent>, &str)> {
        Self::parse(text)
            .into_iter()
            .map(|(component, remainder)| {
                (Arc::new(component) as Arc<dyn QueryComponent>, remainder)
            })
            .collect()
    }
}

impl QueryComponent for IntersectionComponent {
    fn text(&self) -> &str {
        &self.text
    }

    fn penalty_mult(&self) -> f32 {
        f32::min(self.road1.penalty_mult(), self.road2.penalty_mult()) * 5.0f32
    }

    fn debug_name(&self) -> &'static str {
        "IntersectionComponent"
    }

    fn component_type(&self) -> QueryComponentType {
        QueryComponentType::IntersectionComponent
    }

    fn subcomponents(&self) -> Vec<Arc<dyn QueryComponent>> {
        vec![
            Arc::new(self.road1.clone()),
            Arc::new(self.intersection_join_word.clone()),
            Arc::new(self.road2.clone()),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PlaceNameComponent {
    text: String,
}

impl PlaceNameComponent {
    fn new(text: String) -> Self {
        Self { text }
    }

    pub fn parse(text: &str) -> Vec<(Self, &str)> {
        let mut scenarios = Vec::new();
        let mut substring_len = if let Ok((_, token)) = query_term(text) {
            token.len()
        } else {
            return scenarios;
        };

        scenarios.push((
            Self::new(text[..substring_len].to_string()),
            &text[substring_len..],
        ));

        let mut sep_len = if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
            sep.len()
        } else {
            return scenarios;
        };

        loop {
            substring_len += if let Ok((_, token)) = query_term(&text[substring_len + sep_len..]) {
                if token.is_empty() {
                    break;
                }
                token.len()
            } else {
                break;
            };
            substring_len += sep_len;
            scenarios.push((
                Self::new(text[..substring_len].to_string()),
                &text[substring_len..],
            ));
            if let Ok((_, sep)) = query_sep(&text[substring_len..]) {
                sep_len = sep.len();
            } else {
                break;
            }
        }
        return scenarios;
    }

    pub fn parse_boxed(text: &str) -> Vec<(Arc<dyn QueryComponent>, &str)> {
        Self::parse(text)
            .into_iter()
            .map(|(component, remainder)| {
                (Arc::new(component) as Arc<dyn QueryComponent>, remainder)
            })
            .collect()
    }
}

impl QueryComponent for PlaceNameComponent {
    fn text(&self) -> &str {
        &self.text
    }

    fn penalty_mult(&self) -> f32 {
        if brick_and_mortar_words().contains(&self.text.to_lowercase()) {
            1.1f32
        } else {
            0.99f32.powi(self.text.split_whitespace().count() as i32)
        }
    }

    fn debug_name(&self) -> &'static str {
        "PlaceNameComponent"
    }

    fn component_type(&self) -> QueryComponentType {
        QueryComponentType::PlaceNameComponent
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ComponentParser {
    pub(crate) function: fn(&str) -> Vec<(Arc<dyn QueryComponent>, &str)>,
}

lazy_static! {
    pub(crate) static ref COMPONENT_PARSERS: Vec<ComponentParser> = vec![
        ComponentParser {
            function: CategoryComponent::parse_boxed,
        },
        ComponentParser {
            function: NearComponent::parse_boxed,
        },
        ComponentParser {
            function: HouseNumberComponent::parse_boxed,
        },
        ComponentParser {
            function: RoadComponent::parse_boxed,
        },
        ComponentParser {
            function: IntersectionComponent::parse_boxed,
        },
        ComponentParser {
            function: SublocalityComponent::parse_boxed,
        },
        ComponentParser {
            function: LocalityComponent::parse_boxed,
        },
        ComponentParser {
            function: RegionComponent::parse_boxed,
        },
        ComponentParser {
            function: CountryComponent::parse_boxed,
        },
        ComponentParser {
            function: PlaceNameComponent::parse_boxed,
        },
        ComponentParser {
            function: IntersectionJoinWordComponent::parse_boxed,
        },
    ];
}

#[cfg(test)]
mod test {
    use crate::component::IntersectionComponent;
    use test_log::test;

    use super::{CategoryComponent, QueryComponent};

    #[test]
    fn test_category() {
        let text = "grocery store";
        let scenarios = CategoryComponent::parse(text);
        dbg!(&scenarios);
        assert_eq!(scenarios.len(), 1);
        let (component, remainder) = &scenarios[0];
        assert_eq!(remainder, &"");
        assert_eq!(component.text(), "grocery store");
    }

    #[test]
    fn test_category_incomplete_substring() {
        let text = "grocery";
        assert!(CategoryComponent::parse(text).is_empty())
    }

    #[test]
    fn test_road() {
        let text = "main st";
        let scenarios = super::RoadComponent::parse(text);
        assert_eq!(scenarios.len(), 1);
        let (component, remainder) = &scenarios[0];
        assert_eq!(remainder, &"");
        assert_eq!(component.text(), "main st");
    }

    #[test]
    fn test_road_without_suffix() {
        let text = "main";
        let scenarios = super::RoadComponent::parse(text);
        assert_eq!(scenarios.len(), 1);
        let (component, remainder) = &scenarios[0];
        assert_eq!(remainder, &"");
        assert_eq!(component.text(), "main");
        // Exact value may change and is an implementation detail.
        assert!(component.penalty_mult() < 1.0f32);
    }

    #[test]
    fn test_intersection() {
        let text = "fremont ave and n 34th st";
        let mut components = IntersectionComponent::parse(text);
        // assert_eq!(components.len(), 2);
        components.sort_unstable_by(|(a, _), (b, _)| {
            b.penalty_mult().partial_cmp(&a.penalty_mult()).unwrap()
        });
        let (component, remainder) = &components[0];
        assert_eq!(remainder, &"");
        assert_eq!(component.text(), "fremont ave and n 34th st");
        assert_eq!(component.road1().text(), "fremont ave");
        assert_eq!(component.road2().text(), "n 34th st");
        assert_eq!(component.intersection_join_word().text(), "and");
    }

    #[test]
    fn test_intersection_no_suffixes() {
        let text = "union and madison";
        let (component, remainder) = IntersectionComponent::parse(text).pop().unwrap();
        assert_eq!(remainder, "");
        assert_eq!(component.text(), "union and madison");
        assert_eq!(component.road1().text(), "union");
        assert_eq!(component.road2().text(), "madison");
        assert_eq!(component.intersection_join_word().text(), "and");
    }

    #[test]
    fn test_locality() {
        let text = "seattle";
        let scenarios = super::LocalityComponent::parse(text);
        assert_eq!(scenarios.len(), 1);
        let (component, remainder) = &scenarios[0];
        assert_eq!(remainder, &"");
        assert_eq!(component.text(), "seattle");
    }
}
