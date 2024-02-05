#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CuisineCategory {
    /// African cuisine.
    African,
    /// American cuisine.
    American,
    /// Asian cuisine.
    Asian,
    /// European cuisine.
    European,
    /// Middle Eastern cuisine.
    MiddleEastern,
    /// Other cuisine.
    Other { raw_tag: String },
}

impl CuisineCategory {
    pub fn to_facet(&self) -> String {
        match self {
            CuisineCategory::African => "african".to_string(),
            CuisineCategory::American => "american".to_string(),
            CuisineCategory::Asian => "asian".to_string(),
            CuisineCategory::European => "european".to_string(),
            CuisineCategory::MiddleEastern => "middle_eastern".to_string(),
            CuisineCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    /// A place to buy convenience goods.
    Convenience,
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
    /// A place to buy video games.
    VideoGame,
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
            ShopPoiCategory::Convenience => "convenience".to_string(),
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
            ShopPoiCategory::VideoGame => "video_game".to_string(),
            ShopPoiCategory::Other { raw_tag } => {
                format!("other/{}", deunicode::deunicode(raw_tag))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
}
