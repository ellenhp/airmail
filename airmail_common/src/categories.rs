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

pub enum FoodPoiCategory {
    /// A place to buy baked goods.
    Bakery,
    /// A place to buy beverages.
    Beverage,
    /// A place to buy groceries.
    Grocery,
    /// A restaurant or cafe.
    Restaurant,
    /// Other food shop.
    Other { raw_tag: String },
}

pub enum ShopPoiCategory {
    /// A place to buy art.
    Art,
    /// A place to buy books.
    Books,
    /// A place to buy clothes.
    Clothes,
    /// A place to buy convenience goods.
    Convenience,
    /// A place to buy electronics.
    Electronics,
    /// A place to buy flowers.
    Florist,
    /// A place to buy food.
    Food(),
    /// A place to buy furniture.
    Furniture,
    /// A place to buy garden supplies.
    GardenCentre,
    /// A place to buy gifts.
    Gift,
    /// A place to buy hardware.
    Hardware,
    /// A place to buy health supplies.
    Health,
    /// A place to buy jewelry.
    Jewelry,
    /// A place to buy laundry supplies.
    Laundry,
    /// A place to buy liquor.
    Liquor,
    /// A place to buy music.
    Music,
    /// A place to buy news.
    Newsagent,
    /// A place to buy pet supplies.
    Pet,
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
    /// A place to buy video games.
    Video,
    /// Other shop.
    Other { raw_tag: String },
}

pub enum PoiCategory {
    /// An address without additional information, e.g. from OpenAddresses or an untagged OSM node.
    Address,
    /// An administrative area, e.g. a country, state, or city.
    AdminArea,
    /// A place to go in an emergency, e.g. a fire station or hospital.
    Emergency,
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
