use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Write},
};

struct FstBuildHelper {
    input_files: Vec<String>,
}

impl FstBuildHelper {
    pub fn build_fst(
        &mut self,
        dict_file: &str,
        out_file: &str,
        apply_suffixes: &[&str],
        apply_substitutions: &[(&str, &str)],
    ) {
        self.input_files.push(dict_file.to_string());
        // Suffixes must be sorted for the FST creation to succeed.
        let mut apply_suffixes = apply_suffixes
            .iter()
            .map(|s| s.to_lowercase())
            .collect::<Vec<_>>();
        apply_suffixes.sort();

        let mut builder = fst::SetBuilder::memory();
        let file = File::open(dict_file).unwrap();
        let reader = BufReader::new(file);
        let mut lines = BTreeSet::new();
        for result in reader.lines() {
            let line = deunicode::deunicode(result.unwrap().trim()).to_lowercase();
            if apply_substitutions.is_empty() {
                lines.insert(line.clone());
                for suffix in &apply_suffixes {
                    let line = format!("{}{}", &line, suffix);
                    lines.insert(line.clone());
                }
                continue;
            }
            for (from, to) in apply_substitutions {
                let line = line.replace(from, to);
                lines.insert(line.clone());
                for suffix in &apply_suffixes {
                    let line = format!("{}{}", &line, suffix);
                    lines.insert(line.clone());
                }
            }
        }
        for line in lines {
            if line.is_empty() {
                continue;
            }
            builder.insert(line).unwrap();
        }
        let data = builder.into_set().into_fst().into_inner();
        let mut file = File::create(out_file).unwrap();
        file.write_all(&data).unwrap();
    }
}

fn main() {
    let mut helper = FstBuildHelper {
        input_files: Vec::new(),
    };
    helper.build_fst(
        "dicts/en/lp_street_suffixes.txt",
        "dicts/en/lp_street_suffixes.fst",
        &[
            " north",
            " n",
            " south",
            " s",
            " east",
            " e",
            " west",
            " w",
            " northwest",
            " nw",
            " northeast",
            " ne",
            " southwest",
            " sw",
            " southeast",
            " se",
        ],
        &[],
    );
    helper.build_fst(
        "dicts/en/wof_localities.txt",
        "dicts/en/wof_localities.fst",
        &[],
        &[],
    );
    helper.build_fst(
        "dicts/en/wof_regions.txt",
        "dicts/en/wof_regions.fst",
        &[],
        &[],
    );
    helper.build_fst(
        "dicts/en/wof_countries.txt",
        "dicts/en/wof_countries.fst",
        &[],
        &[],
    );
    helper.build_fst("dicts/en/near.txt", "dicts/en/near.fst", &[], &[]);
    helper.build_fst("dicts/en/category.txt", "dicts/en/category.fst", &[], &[]);
    helper.build_fst(
        "dicts/en/intersection_join.txt",
        "dicts/en/intersection_join.fst",
        &[],
        &[],
    );
    helper.build_fst(
        "dicts/en/brick_and_mortar.txt",
        "dicts/en/brick_and_mortar.fst",
        &[],
        &[
            (" & ", " and "),
            ("'", ""),
            ("-", " "),
            ("-", ""),
            ("(", ""),
            (")", ""),
            (",", " "),
            ("!", ""),
            (",", " "),
            ("#", " "),
        ],
    );

    println!(
        "cargo:rerun-if-changed=build.rs,{}",
        helper.input_files.join(",")
    );
}
