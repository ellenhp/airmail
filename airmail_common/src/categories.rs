use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum AmenityPoiCategory {
    /// A public toilet or shower.
    Toilets,
    /// A public shelter, e.g. a bus shelter or a picnic shelter.
    Shelter,
    /// Public water source.
    DrinkingWater,
    /// A public telephone.
    Telephone,
    /// A public library.
    Library,
}

impl AmenityPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            AmenityPoiCategory::Toilets => "toilets".to_string(),
            AmenityPoiCategory::Shelter => "shelter".to_string(),
            AmenityPoiCategory::DrinkingWater => "drinking_water".to_string(),
            AmenityPoiCategory::Telephone => "telephone".to_string(),
            AmenityPoiCategory::Library => "library".to_string(),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            AmenityPoiCategory::Toilets => vec![
                "toilets".to_string(),
                "restroom".to_string(),
                "washroom".to_string(),
                "bathroom".to_string(),
                "loo".to_string(),
                "wash closet".to_string(),
            ],
            AmenityPoiCategory::Shelter => vec!["shelter".to_string()],
            AmenityPoiCategory::DrinkingWater => vec![
                "drinking water".to_string(),
                "water".to_string(),
                "fountain".to_string(),
                "spigot".to_string(),
            ],
            AmenityPoiCategory::Telephone => vec!["telephone".to_string()],
            AmenityPoiCategory::Library => {
                vec!["library".to_string(), "public library".to_string()]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum NaturalPoiCategory {
    /// A mountain, hill, or other point of elevation.
    Peak,
    /// A body of water, e.g. a lake or river.
    Water,
    /// A forest, park, or other area of trees.
    Wood,
    /// A natural feature that is not a peak, water, or wood.
    Other { raw_tag: String },
}

impl NaturalPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            NaturalPoiCategory::Peak => "peak".to_string(),
            NaturalPoiCategory::Water => "water".to_string(),
            NaturalPoiCategory::Wood => "wood".to_string(),
            NaturalPoiCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            NaturalPoiCategory::Peak => vec!["peak".to_string(), "mountain".to_string()],
            NaturalPoiCategory::Water => vec![
                "water".to_string(),
                "lake".to_string(),
                "river".to_string(),
                "stream".to_string(),
                "pond".to_string(),
            ],
            NaturalPoiCategory::Wood => vec!["forest".to_string()],
            NaturalPoiCategory::Other { raw_tag: _ } => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TransitPoiCategory {
    /// A bus stop.
    BusStop,
    /// A train station.
    TrainStation,
    /// An airport.
    Airport,
    /// A ferry terminal.
    FerryTerminal,
    /// A subway station.
    SubwayStation,
    /// A tram stop.
    TramStop,
    /// Other transit feature.
    Other { raw_tag: String },
}

impl TransitPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            TransitPoiCategory::BusStop => "bus_stop".to_string(),
            TransitPoiCategory::TrainStation => "train_station".to_string(),
            TransitPoiCategory::Airport => "airport".to_string(),
            TransitPoiCategory::FerryTerminal => "ferry_terminal".to_string(),
            TransitPoiCategory::SubwayStation => "subway_station".to_string(),
            TransitPoiCategory::TramStop => "tram_stop".to_string(),
            TransitPoiCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            TransitPoiCategory::BusStop => vec![
                "bus stop".to_string(),
                "bus station".to_string(),
                "bus".to_string(),
            ],
            TransitPoiCategory::TrainStation => vec![
                "train station".to_string(),
                "train".to_string(),
                "railway station".to_string(),
            ],
            TransitPoiCategory::Airport => vec!["airport".to_string()],
            TransitPoiCategory::FerryTerminal => {
                vec!["ferry terminal".to_string(), "ferry".to_string()]
            }
            TransitPoiCategory::SubwayStation => {
                vec!["subway station".to_string(), "subway".to_string()]
            }
            TransitPoiCategory::TramStop => vec![
                "tram stop".to_string(),
                "tram station".to_string(),
                "tram".to_string(),
            ],
            TransitPoiCategory::Other { raw_tag: _ } => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CuisineCategory {
    /// African cuisine.
    African,
    /// American cuisine.
    American,
    /// Asian cuisine.
    Asian,
    /// Coffee shop or cafe.
    CoffeeShop,
    /// European cuisine.
    European,
    /// Middle Eastern cuisine.
    MiddleEastern,
    /// Pizza
    Pizza,
    /// Other cuisine.
    Other { raw_tag: String },
}

impl CuisineCategory {
    pub fn to_facet(&self) -> String {
        match self {
            CuisineCategory::African => "african".to_string(),
            CuisineCategory::American => "american".to_string(),
            CuisineCategory::Asian => "asian".to_string(),
            CuisineCategory::CoffeeShop => "coffee".to_string(),
            CuisineCategory::European => "european".to_string(),
            CuisineCategory::MiddleEastern => "middle_eastern".to_string(),
            CuisineCategory::Pizza => "pizza".to_string(),
            CuisineCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        let mut values = match self {
            CuisineCategory::African => vec![
                "african".to_string(),
                "african food".to_string(),
                "african restaurant".to_string(),
                "ethiopian".to_string(),
                "ethiopian food".to_string(),
                "ethiopian restaurant".to_string(),
                "moroccan".to_string(),
                "moroccan food".to_string(),
                "moroccan restaurant".to_string(),
            ],
            CuisineCategory::American => vec![
                "american".to_string(),
                "american food".to_string(),
                "american restaurant".to_string(),
                "burger".to_string(),
                "burger joint".to_string(),
                "burger restaurant".to_string(),
                "diner".to_string(),
                "diner food".to_string(),
                "diner restaurant".to_string(),
                "fast food".to_string(),
                "fast food restaurant".to_string(),
                "hot dog".to_string(),
                "hot dog joint".to_string(),
                "hot dog restaurant".to_string(),
                "sandwich".to_string(),
                "sandwich joint".to_string(),
                "sandwich restaurant".to_string(),
            ],
            CuisineCategory::Asian => vec![
                // This is really culturally insensitive of me but I don't have the energy right now to fix it,
                // and it's probably better to conflate these categories than to leave them out entirely.
                // We need something in like a yaml file somewhere translated to a bunch of different languages, long term.
                "asian".to_string(),
                "asian food".to_string(),
                "asian restaurant".to_string(),
                "chinese".to_string(),
                "chinese food".to_string(),
                "chinese restaurant".to_string(),
                "indian".to_string(),
                "indian food".to_string(),
                "indian restaurant".to_string(),
                "japanese".to_string(),
                "japanese food".to_string(),
                "japanese restaurant".to_string(),
                "korean".to_string(),
                "korean food".to_string(),
                "korean restaurant".to_string(),
                "thai".to_string(),
                "thai food".to_string(),
                "thai restaurant".to_string(),
                "vietnamese".to_string(),
                "vietnamese food".to_string(),
                "vietnamese restaurant".to_string(),
            ],
            CuisineCategory::CoffeeShop => vec![
                "coffee".to_string(),
                "coffee shop".to_string(),
                "cafe".to_string(),
            ],
            CuisineCategory::European => vec![
                "european".to_string(),
                "european food".to_string(),
                "european restaurant".to_string(),
            ],
            CuisineCategory::MiddleEastern => {
                vec![
                    "middle eastern".to_string(),
                    "middle eastern food".to_string(),
                    "middle eastern restaurant".to_string(),
                    "mediterranean".to_string(),
                    "mediterranean food".to_string(),
                    "mediterranean restaurant".to_string(),
                ]
            }
            CuisineCategory::Pizza => vec!["pizza".to_string(), "pizzeria".to_string()],
            CuisineCategory::Other { raw_tag: _ } => vec![],
        };
        values.push("restaurant".to_string());
        values.push("food".to_string());
        values
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum EmergencyPoiCategory {
    /// A fire station.
    FireStation,
    /// A hospital.
    Hospital,
    /// A police station.
    PoliceStation,
}

impl EmergencyPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            EmergencyPoiCategory::FireStation => "fire_station".to_string(),
            EmergencyPoiCategory::Hospital => "hospital".to_string(),
            EmergencyPoiCategory::PoliceStation => "police_station".to_string(),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            EmergencyPoiCategory::FireStation => vec!["fire station".to_string()],
            EmergencyPoiCategory::Hospital => vec![
                "hospital".to_string(),
                "emergency room".to_string(),
                "er".to_string(),
            ],
            EmergencyPoiCategory::PoliceStation => {
                vec!["police".to_string(), "police station".to_string()]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum FoodPoiCategory {
    /// A place to buy baked goods.
    Bakery,
    /// A place to buy beverages.
    Beverage,
    /// A place to buy groceries.
    Grocery,
    /// A restaurant or cafe.
    Restaurant(Option<CuisineCategory>),
    /// Other food shop.
    Other { raw_tag: String },
}

impl FoodPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            FoodPoiCategory::Bakery => "bakery".to_string(),
            FoodPoiCategory::Beverage => "beverage".to_string(),
            FoodPoiCategory::Grocery => "grocery".to_string(),
            FoodPoiCategory::Restaurant(Some(cuisine)) => {
                format!("restaurant/{}", cuisine.to_facet())
            }
            FoodPoiCategory::Restaurant(None) => "restaurant".to_string(),
            FoodPoiCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            FoodPoiCategory::Bakery => vec!["bakery".to_string()],
            FoodPoiCategory::Beverage => vec!["beverage".to_string()],
            FoodPoiCategory::Grocery => vec![
                "grocery".to_string(),
                "grocery store".to_string(),
                "supermarket".to_string(),
                "market".to_string(),
                "food".to_string(),
            ],
            FoodPoiCategory::Restaurant(Some(cuisine)) => cuisine.labels(),
            FoodPoiCategory::Restaurant(None) => vec!["restaurant".to_string(), "food".to_string()],
            FoodPoiCategory::Other { raw_tag: _ } => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ShopPoiCategory {
    /// An adult store, e.g. a sex shop, strip club or bathhouse.
    Adult,
    /// A place to buy art.
    Art,
    /// A bank or ATM.
    Bank,
    /// A bar.
    Bar,
    /// A place to buy books.
    Books,
    /// A doctor's office.
    Clinic,
    /// A place to buy clothes.
    Clothes,
    /// A coffee shop.
    Coffee,
    /// A place to buy convenience goods.
    Convenience,
    /// A dentist.
    Dentist,
    /// A place to buy electronics.
    Electronics,
    /// A place to buy flowers.
    Florist,
    /// A place to buy food, including restaurants and grocery stores.
    Food(FoodPoiCategory),
    /// A place to buy furniture.
    Furniture,
    /// A place to buy garden supplies.
    Gift,
    /// A hardware store, garden store, or big-box home improvement retailer.
    Hardware,
    /// A place to buy health supplies.
    Health,
    /// A place to buy jewelry.
    Jewelry,
    /// A place to buy liquor (not a bar).
    Liquor,
    /// A place to buy music.
    Music,
    /// A place to buy pet supplies.
    Pet,
    /// A pharmacy.
    Pharmacy,
    /// A place to buy photo supplies.
    Photo,
    /// A place to buy shoes.
    Shoes,
    /// A place to buy sports supplies.
    Sports,
    /// A place to buy tobacco.
    Tobacco,
    /// A place to buy toys.
    Toys,
    /// A veterinarian's office.
    Veterinary,
    /// Other shop.
    Other { raw_tag: String },
}

impl ShopPoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            ShopPoiCategory::Adult => "adult".to_string(),
            ShopPoiCategory::Art => "art".to_string(),
            ShopPoiCategory::Bank => "bank".to_string(),
            ShopPoiCategory::Bar => "bar".to_string(),
            ShopPoiCategory::Books => "books".to_string(),
            ShopPoiCategory::Clothes => "clothes".to_string(),
            ShopPoiCategory::Clinic => "clinic".to_string(),
            ShopPoiCategory::Coffee => "coffee".to_string(),
            ShopPoiCategory::Convenience => "convenience".to_string(),
            ShopPoiCategory::Dentist => "dentist".to_string(),
            ShopPoiCategory::Electronics => "electronics".to_string(),
            ShopPoiCategory::Florist => "florist".to_string(),
            ShopPoiCategory::Food(food) => format!("food/{}", food.to_facet()),
            ShopPoiCategory::Furniture => "furniture".to_string(),
            ShopPoiCategory::Gift => "gift".to_string(),
            ShopPoiCategory::Hardware => "hardware".to_string(),
            ShopPoiCategory::Health => "health".to_string(),
            ShopPoiCategory::Jewelry => "jewelry".to_string(),
            ShopPoiCategory::Liquor => "liquor".to_string(),
            ShopPoiCategory::Music => "music".to_string(),
            ShopPoiCategory::Pet => "pet".to_string(),
            ShopPoiCategory::Pharmacy => "pharmacy".to_string(),
            ShopPoiCategory::Photo => "photo".to_string(),
            ShopPoiCategory::Shoes => "shoes".to_string(),
            ShopPoiCategory::Sports => "sports".to_string(),
            ShopPoiCategory::Tobacco => "tobacco".to_string(),
            ShopPoiCategory::Toys => "toys".to_string(),
            ShopPoiCategory::Veterinary => "veterinary".to_string(),
            ShopPoiCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            ShopPoiCategory::Adult => vec![
                "adult store".to_string(),
                "sex shop".to_string(),
                "strip club".to_string(),
                "bathhouse".to_string(),
            ],
            ShopPoiCategory::Art => vec!["art".to_string(), "art store".to_string()],
            ShopPoiCategory::Bank => vec!["bank".to_string(), "atm".to_string()],
            ShopPoiCategory::Bar => vec![
                "bar".to_string(),
                "pub".to_string(),
                "tavern".to_string(),
                "saloon".to_string(),
                "taproom".to_string(),
                "beer hall".to_string(),
                "beer garden".to_string(),
                "brewery".to_string(),
            ],
            ShopPoiCategory::Books => vec![
                "books".to_string(),
                "bookstore".to_string(),
                "book shop".to_string(),
            ],
            ShopPoiCategory::Clothes => vec!["clothes".to_string(), "clothing".to_string()],
            ShopPoiCategory::Clinic => vec![
                "clinic".to_string(),
                "doctor".to_string(),
                "doctor's office".to_string(),
                "doctors office".to_string(),
                "doctors".to_string(),
            ],
            ShopPoiCategory::Coffee => vec![
                "coffee".to_string(),
                "coffee shop".to_string(),
                "cafe".to_string(),
                "coffeehouse".to_string(),
                "coffeeshop".to_string(),
            ],
            ShopPoiCategory::Convenience => {
                vec!["convenience".to_string(), "convenience store".to_string()]
            }
            ShopPoiCategory::Dentist => vec![
                "dentist".to_string(),
                "dental".to_string(),
                "dental office".to_string(),
                "dental clinic".to_string(),
                "dental care".to_string(),
            ],
            ShopPoiCategory::Electronics => vec!["electronics".to_string()],
            ShopPoiCategory::Florist => vec![
                "florist".to_string(),
                "flower shop".to_string(),
                "flowers".to_string(),
            ],
            ShopPoiCategory::Food(food) => food.labels(),
            ShopPoiCategory::Furniture => vec!["furniture".to_string()],
            ShopPoiCategory::Gift => vec!["gift".to_string()],
            ShopPoiCategory::Hardware => vec![
                "hardware".to_string(),
                "hardware store".to_string(),
                "home improvement".to_string(),
            ],
            ShopPoiCategory::Health => vec!["health".to_string()],
            ShopPoiCategory::Jewelry => vec!["jewelry".to_string()],
            ShopPoiCategory::Liquor => vec!["liquor".to_string()],
            ShopPoiCategory::Music => vec!["music".to_string()],
            ShopPoiCategory::Pet => vec![
                "pet".to_string(),
                "pet store".to_string(),
                "pets".to_string(),
                "pet supplies".to_string(),
                "cat food".to_string(),
                "dog food".to_string(),
                "cat litter".to_string(),
            ],
            ShopPoiCategory::Pharmacy => vec!["pharmacy".to_string(), "drugstore".to_string()],
            ShopPoiCategory::Photo => vec![
                "photo".to_string(),
                "photo store".to_string(),
                "photography".to_string(),
                "camera".to_string(),
                "film".to_string(),
                "photo lab".to_string(),
            ],
            ShopPoiCategory::Shoes => vec![
                "shoes".to_string(),
                "shoe store".to_string(),
                "footwear".to_string(),
            ],
            ShopPoiCategory::Sports => vec![
                "sports".to_string(),
                "sporting goods".to_string(),
                "sporting goods store".to_string(),
            ],
            ShopPoiCategory::Tobacco => vec![
                "tobacco".to_string(),
                "tobacco store".to_string(),
                "smoke shop".to_string(),
            ],
            ShopPoiCategory::Toys => vec!["toys".to_string(), "toy store".to_string()],
            ShopPoiCategory::Veterinary => vec![
                "veterinary".to_string(),
                "veterinarian".to_string(),
                "vet".to_string(),
                "vet clinic".to_string(),
                "veterinary hospital".to_string(),
                "animal hospital".to_string(),
            ],
            ShopPoiCategory::Other { raw_tag: _ } => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PoiCategory {
    /// An address without additional information, e.g. from OpenAddresses or an untagged OSM node.
    Address,
    /// An administrative area, e.g. a country, state, or city.
    AdminArea,
    /// Amenities for public use, e.g. public toilets, drinking fountains.
    Amenity(AmenityPoiCategory),
    /// A place to go in an emergency, e.g. a fire station or hospital.
    Emergency(EmergencyPoiCategory),
    /// A road or path.
    Highway,
    /// Land use, e.g. a park or a school.
    Landuse,
    /// A place to stay, e.g. a hotel or campsite.
    Leisure,
    /// A natural feature, e.g. a mountain or lake.
    Natural(NaturalPoiCategory),
    /// A transportation feature, e.g. a bus stop, airport, or train station.
    Transit(TransitPoiCategory),
    /// A place that exists to sell physical goods, e.g. a shop or restaurant.
    Shop(ShopPoiCategory),
    /// A sports facility, e.g. a golf course or stadium.
    Sport,
    /// A tourist attraction, e.g. a museum or viewpoint.
    Tourism,
}

impl PoiCategory {
    pub fn to_facet(&self) -> String {
        match self {
            PoiCategory::Address => "/address".to_string(),
            PoiCategory::AdminArea => "/admin_area".to_string(),
            PoiCategory::Amenity(amenity) => format!("/amenity/{}", amenity.to_facet()),
            PoiCategory::Emergency(emergency) => format!("/emergency/{}", emergency.to_facet()),
            PoiCategory::Highway => "/highway".to_string(),
            PoiCategory::Landuse => "/landuse".to_string(),
            PoiCategory::Leisure => "/leisure".to_string(),
            PoiCategory::Natural(natural) => format!("/natural/{}", natural.to_facet()),
            PoiCategory::Transit(transit) => format!("/transit/{}", transit.to_facet()),
            PoiCategory::Shop(shop) => format!("/shop/{}", shop.to_facet()),
            PoiCategory::Sport => "/sport".to_string(),
            PoiCategory::Tourism => "/tourism".to_string(),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        match self {
            PoiCategory::Amenity(amenity) => amenity.labels(),
            PoiCategory::Emergency(emergency) => emergency.labels(),
            PoiCategory::Natural(natural) => natural.labels(),
            PoiCategory::Transit(transit) => transit.labels(),
            PoiCategory::Shop(shop) => shop.labels(),
            _ => vec![],
        }
    }
}
