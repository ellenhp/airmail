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
        let components_scenarios = Self::parse_recurse(&[], input);
        debug!("Found {} scenarios", components_scenarios.len());
        let mut scored_scenarios = components_scenarios
            .iter()
            .map(|scenario| (scenario, scenario.penalty_mult()))
            .collect::<Vec<_>>();
        scored_scenarios.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Self {
            components_scenarios: scored_scenarios
                .iter()
                .map(|(scenario, _score)| (*scenario).clone())
                .collect(),
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
            scenario.components[0].as_ref().debug_name(),
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
            scenario.components[0].as_ref().debug_name(),
            "HouseNumberComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "RoadComponent"
        );
        assert_eq!(
            scenario.components[2].as_ref().debug_name(),
            "LocalityComponent"
        );
        assert_eq!(
            scenario.components[3].as_ref().debug_name(),
            "RegionComponent"
        );
        assert_eq!(
            scenario.components[4].as_ref().debug_name(),
            "CountryComponent"
        );
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
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "LocalityComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "RegionComponent"
        );
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
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "PlaceNameComponent"
        );
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
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "PlaceNameComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "LocalityComponent"
        );
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
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "LocalityComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "PlaceNameComponent"
        );
    }

    #[test]
    fn parse_burger_place() {
        let now = Instant::now();
        let query = Query::parse("next level");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 1);
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "PlaceNameComponent"
        );
    }

    #[test]
    fn sublocality_penalized_over_road_suffix() {
        let now = Instant::now();
        let query = Query::parse("100 fremont avenue north seattle");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 3);
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "HouseNumberComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "RoadComponent"
        );
        assert_eq!(
            scenario.components[2].as_ref().debug_name(),
            "LocalityComponent"
        );
    }

    #[test]
    fn sublocality() {
        let now = Instant::now();
        let query = Query::parse("food in downtown seattle");
        println!("took {:?}", now.elapsed());
        let scenarios = query.scenarios();
        let scenario = scenarios.iter().next().unwrap();
        dbg!(&scenario);
        assert_eq!(scenario.components.len(), 4);
        assert_eq!(
            scenario.components[0].as_ref().debug_name(),
            "CategoryComponent"
        );
        assert_eq!(
            scenario.components[1].as_ref().debug_name(),
            "NearComponent"
        );
        assert_eq!(
            scenario.components[2].as_ref().debug_name(),
            "SublocalityComponent"
        );
        assert_eq!(
            scenario.components[3].as_ref().debug_name(),
            "LocalityComponent"
        );
    }
}
