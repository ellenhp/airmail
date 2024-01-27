use crate::query::QueryScenario;

pub struct QueryScenarioScorer {
    score_mult: fn(query: &QueryScenario) -> f32,
}

inventory::collect!(QueryScenarioScorer);

impl QueryScenarioScorer {
    pub fn score(&self, scenario: &QueryScenario) -> f32 {
        (self.score_mult)(scenario)
    }
}

// Penalizing multiple roads in one query is fine because we have a separate component for intersections.
fn max_one_road(scenario: &QueryScenario) -> f32 {
    let mut has_road = false;
    for component in scenario.as_vec() {
        if component.name() == "RoadComponent" {
            if has_road {
                return 0.0;
            }
            has_road = true;
        }
    }
    1.0
}
inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_road,
    }
}

fn max_one_house_num(scenario: &QueryScenario) -> f32 {
    let mut has_house_num = false;
    for component in scenario.as_vec() {
        if component.name() == "HouseNumberComponent" {
            if has_house_num {
                return 0.0;
            }
            has_house_num = true;
        }
    }
    1.0
}
inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_house_num,
    }
}

fn house_num_road_together(scenario: &QueryScenario) -> f32 {
    let mut count = 0;
    for component_of_interest in scenario.as_vec().iter().map(|component| {
        component.name() == "HouseNumberComponent" || component.name() == "RoadComponent"
    }) {
        if component_of_interest {
            count += 1;
        } else {
            if count != 0 && count != 2 {
                return 0.0f32;
            }
        }
    }
    1.0f32
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: house_num_road_together,
    }
}

fn max_one_unit(scenario: &QueryScenario) -> f32 {
    let mut has_unit = false;
    for component in scenario.as_vec() {
        if component.name() == "UnitComponent" {
            if has_unit {
                return 0.0;
            }
            has_unit = true;
        }
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_unit,
    }
}

fn max_one_locality(scenario: &QueryScenario) -> f32 {
    let mut has_locality = false;
    for component in scenario.as_vec() {
        if component.name() == "LocalityComponent" {
            if has_locality {
                return 0.0;
            }
            has_locality = true;
        }
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_locality,
    }
}

fn max_one_region(scenario: &QueryScenario) -> f32 {
    let mut has_region = false;
    for component in scenario.as_vec() {
        if component.name() == "RegionComponent" {
            if has_region {
                return 0.0;
            }
            has_region = true;
        }
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_region,
    }
}

fn max_one_country(scenario: &QueryScenario) -> f32 {
    let mut has_country = false;
    for component in scenario.as_vec() {
        if component.name() == "CountryComponent" {
            if has_country {
                return 0.0;
            }
            has_country = true;
        }
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: max_one_country,
    }
}

fn country_not_before_locality(scenario: &QueryScenario) -> f32 {
    let mut has_locality = false;
    let mut country_first = false;
    for component in scenario.as_vec() {
        if component.name() == "CountryComponent" {
            if !has_locality {
                country_first = true;
            }
        }
        if component.name() == "LocalityComponent" {
            has_locality = true;
        }
    }
    if country_first {
        return 0.0;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: country_not_before_locality,
    }
}

fn region_not_before_locality(scenario: &QueryScenario) -> f32 {
    let mut has_locality = false;
    let mut region_first = false;
    for component in scenario.as_vec() {
        if component.name() == "RegionComponent" {
            if !has_locality {
                region_first = true;
            }
        }
        if component.name() == "LocalityComponent" {
            has_locality = true;
        }
    }
    if region_first {
        return 0.0;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: region_not_before_locality,
    }
}

fn country_not_before_region(scenario: &QueryScenario) -> f32 {
    let mut has_region = false;
    let mut country_first = false;
    for component in scenario.as_vec() {
        if component.name() == "CountryComponent" {
            if !has_region {
                country_first = true;
            }
        }
        if component.name() == "RegionComponent" {
            has_region = true;
        }
    }
    if country_first {
        return 0.0;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: country_not_before_region,
    }
}

fn naked_road_unlikely(scenario: &QueryScenario) -> f32 {
    let mut has_road = false;
    let mut has_house_num = false;
    for component in scenario.as_vec() {
        if component.name() == "RoadComponent" {
            has_road = true;
        }
        if component.name() == "HouseNumberComponent" {
            has_house_num = true;
        }
    }
    if has_road && !has_house_num {
        return 0.2;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: naked_road_unlikely,
    }
}

fn no_naked_house_num(scenario: &QueryScenario) -> f32 {
    let mut has_road = false;
    let mut has_house_num = false;
    for component in scenario.as_vec() {
        if component.name() == "RoadComponent" {
            has_road = true;
        }
        if component.name() == "HouseNumberComponent" {
            has_house_num = true;
        }
    }
    // We can't return zero here otherwise it'll exit early.
    if !has_road && has_house_num {
        return 0.01;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: no_naked_house_num,
    }
}

fn no_naked_unit(scenario: &QueryScenario) -> f32 {
    let mut has_road = false;
    let mut has_unit = false;
    for component in scenario.as_vec() {
        if component.name() == "RoadComponent" {
            has_road = true;
        }
        if component.name() == "UnitComponent" {
            has_unit = true;
        }
    }
    if !has_road && has_unit {
        return 0.0;
    }
    1.0
}

inventory::submit! {
    QueryScenarioScorer {
        score_mult: no_naked_unit,
    }
}

pub fn score_scenario(scenario: &QueryScenario) -> f32 {
    let mut score = 1.0;
    for scorer in inventory::iter::<QueryScenarioScorer> {
        score *= scorer.score(scenario);
    }
    score
}
