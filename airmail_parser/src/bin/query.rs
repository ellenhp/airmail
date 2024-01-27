use airmail_parser::query::Query;

fn main() {
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = Query::parse("broadway ave, seattle");
        let _ = Query::parse("helsinki, finland");
        let _ = Query::parse("1600 pennsylvania ave");
        let _ = Query::parse("123 main st, st louis, missouri, united states");
        let _ = Query::parse("seattle, wa");
        let _ = Query::parse("trader joe's in sacramento, ca");
        let _ = Query::parse("boylston and denny");
        let _ = Query::parse("bowling near downtown everett, washington, united states");
        let _ = Query::parse("fred meyer");
        let _ = Query::parse("fred meyer in seattle, wa, united states");
    }
    println!("took {:?}", start.elapsed());
}
