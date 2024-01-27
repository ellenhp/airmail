use std::sync::Arc;

use log::debug;

use crate::{
    common::query_sep,
    component::{QueryComponent, COMPONENT_PARSERS},
    scorers::score_scenario,
};

#[derive(Debug, Clone)]
pub struct QueryScenario {
    components: Vec<Arc<dyn QueryComponent>>,
}

impl QueryScenario {
    pub fn penalty_mult(&self) -> f32 {
        score_scenario(self)
            * self
                .components
                .iter()
                .map(|component| component.penalty_mult())
                .product::<f32>()
    }

    pub fn as_vec(&self) -> Vec<&dyn QueryComponent> {
        self.components.iter().map(|c| c.as_ref()).collect()
    }
}

pub struct Query {
    components_scenarios: Vec<QueryScenario>,
}

impl Query {
    fn parse_recurse(prefix: &[Arc<dyn QueryComponent>], remaining: &str) -> Vec<QueryScenario> {
        if score_scenario(&QueryScenario {
            components: prefix.to_vec(),
        }) == 0.0
        {
            return Vec::new();
        }
        let mut scenarios = Vec::new();
        if remaining.is_empty() {
            scenarios.push(QueryScenario {
                components: prefix.to_vec(),
            });
        } else {
            for component_parser in COMPONENT_PARSERS.iter() {
                for (new_component, new_remaining) in (component_parser.function)(remaining) {
                    let mut new_prefix = prefix.to_vec();
                    new_prefix.push(new_component);
                    // Remove any leading separators.
                    let new_remaining = if let Ok((new_remaining, _sep)) = query_sep(new_remaining)
                    {
                        new_remaining
                    } else {
                        new_remaining
                    };
                    scenarios.extend(Self::parse_recurse(&new_prefix, new_remaining));
                }
            }
        }
        scenarios
    }

    pub fn parse(input: &str) -> Self {
        debug!("Parsing query: {:?}", input);
        let mut components_scenarios = Self::parse_recurse(&[], input);
        components_scenarios
            .sort_by(|a, b| b.penalty_mult().partial_cmp(&a.penalty_mult()).unwrap());
        Self {
            components_scenarios,
        }
    }

    pub fn scenarios(&self) -> Vec<QueryScenario> {
        self.components_scenarios.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn test_parse_intersection() {
        let now = Instant::now();
        let query = Query::parse("boylston and denny");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 1);
        assert_eq!(
            scenario.components[0].as_ref().name(),
            "IntersectionComponent"
        );
    }

    #[test]
    fn test_parse_address() {
        let now = Instant::now();
        let query = Query::parse("123 main st, st louis, missouri, united states");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 5);
        assert_eq!(
            scenario.components[0].as_ref().name(),
            "HouseNumberComponent"
        );
        assert_eq!(scenario.components[1].as_ref().name(), "RoadComponent");
        assert_eq!(scenario.components[2].as_ref().name(), "LocalityComponent");
        assert_eq!(scenario.components[3].as_ref().name(), "RegionComponent");
        assert_eq!(scenario.components[4].as_ref().name(), "CountryComponent");
    }

    #[test]
    fn test_parse_locality_region() {
        let now = Instant::now();
        let query = Query::parse("seattle, wa");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 2);
        assert_eq!(scenario.components[0].as_ref().name(), "LocalityComponent");
        assert_eq!(scenario.components[1].as_ref().name(), "RegionComponent");
    }

    #[test]
    fn test_place_name() {
        let now = Instant::now();
        let query = Query::parse("fred meyer");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 1);
        assert_eq!(scenario.components[0].as_ref().name(), "PlaceNameComponent");
    }

    #[test]
    fn test_place_name_with_locality() {
        let now = Instant::now();
        let query = Query::parse("fred meyer seattle");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 2);
        assert_eq!(scenario.components[0].as_ref().name(), "PlaceNameComponent");
        assert_eq!(scenario.components[1].as_ref().name(), "LocalityComponent");
    }

    #[test]
    fn test_place_name_with_locality_reversed() {
        let now = Instant::now();
        let query = Query::parse("seattle fred meyer");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 2);
        assert_eq!(scenario.components[0].as_ref().name(), "LocalityComponent");
        assert_eq!(scenario.components[1].as_ref().name(), "PlaceNameComponent");
    }
}