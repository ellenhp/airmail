use crate::query::QueryScenario;

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
    if country_first && has_locality {
        return 0.0;
    }
    1.0
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
    if region_first && has_locality {
        return 0.0;
    }
    1.0
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
    if country_first && has_region {
        return 0.0;
    }
    1.0
}

fn housenum_not_before_placename(scenario: &QueryScenario) -> f32 {
    let mut has_placename = false;
    let mut housenum_first = false;
    for component in scenario.as_vec() {
        if component.name() == "HouseNumberComponent" {
            if !has_placename {
                housenum_first = true;
            }
        }
        if component.name() == "PlaceNameComponent" {
            has_placename = true;
        }
    }
    if housenum_first && has_placename {
        return 0.01;
    }
    1.0
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
        return 0.05;
    }
    1.0
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
        return 0.01;
    }
    1.0
}

fn sublocality_must_preceed_locality(scenario: &QueryScenario) -> f32 {
    let mut last_is_sublocality = false;
    for component in scenario.as_vec() {
        if last_is_sublocality && component.name() != "LocalityComponent" {
            return 0.01;
        }
        if component.name() == "SubLocalityComponent" {
            last_is_sublocality = true;
        } else {
            last_is_sublocality = false;
        }
    }
    1.0
}

// "On" and "In" are both country/region codes too.
fn near_not_last_if_not_category(scenario: &QueryScenario) -> f32 {
    let mut components = scenario.as_vec();
    if let Some(component) = components.pop() {
        if component.name() != "NearComponent" {
            return 1.0;
        }
    }
    if let Some(component) = components.pop() {
        if component.name() != "CategoryComponent" {
            return 0.01;
        }
    }
    1.0
}

pub struct QueryScenarioScorer {
    score_mult: fn(query: &QueryScenario) -> f32,
}

impl QueryScenarioScorer {
    pub fn score(&self, scenario: &QueryScenario) -> f32 {
        (self.score_mult)(scenario)
    }
}

lazy_static! {
    pub static ref QUERY_SCENARIO_SCORERS: Vec<QueryScenarioScorer> = vec![
        QueryScenarioScorer {
            score_mult: max_one_road,
        },
        QueryScenarioScorer {
            score_mult: max_one_house_num,
        },
        QueryScenarioScorer {
            score_mult: house_num_road_together,
        },
        QueryScenarioScorer {
            score_mult: max_one_unit,
        },
        QueryScenarioScorer {
            score_mult: max_one_locality,
        },
        QueryScenarioScorer {
            score_mult: max_one_region,
        },
        QueryScenarioScorer {
            score_mult: max_one_country,
        },
        QueryScenarioScorer {
            score_mult: country_not_before_locality,
        },
        QueryScenarioScorer {
            score_mult: region_not_before_locality,
        },
        QueryScenarioScorer {
            score_mult: country_not_before_region,
        },
        QueryScenarioScorer {
            score_mult: housenum_not_before_placename,
        },
        QueryScenarioScorer {
            score_mult: naked_road_unlikely,
        },
        QueryScenarioScorer {
            score_mult: no_naked_house_num,
        },
        QueryScenarioScorer {
            score_mult: no_naked_unit,
        },
        QueryScenarioScorer {
            score_mult: sublocality_must_preceed_locality,
        },
        QueryScenarioScorer {
            score_mult: near_not_last_if_not_category,
        },
    ];
}

pub fn score_scenario(scenario: &QueryScenario) -> f32 {
    let mut score = 1.0;
    for scorer in QUERY_SCENARIO_SCORERS.iter() {
        score *= scorer.score(scenario);
    }
    score
}
