use solana_pubkey::Pubkey;
use solana_svm::transaction_balances::{DatumInclusion, ProgramDatumInclusions};
use std::{collections::HashMap, fs, path::PathBuf};

pub enum PreOrPostDatum {
    PreDatum,
    PostDatum,
}

pub type InclusionsFromConfig = HashMap<String, DatumInclusion>;

#[derive(serde::Deserialize)]
struct GeyserConfigFileWithInclusions {
    #[serde(rename = "datumProgramInclusions")]
    pub datum_program_inclusions: InclusionsFromConfig,
}

pub fn load_datum_program_inclusions(paths: &Option<Vec<PathBuf>>) -> ProgramDatumInclusions {
    let mut datum_program_inclusions: ProgramDatumInclusions = HashMap::new();
    if let Some(paths) = paths {
        for path in paths {
            let file = fs::read(path);
            if !file.is_ok() {
                eprintln!("Unable to read JSON file: {:?} Skipping...", path);
                continue;
            }
            let json = serde_json::from_slice::<GeyserConfigFileWithInclusions>(&file.unwrap());
            if let Err(e) = json {
                eprintln!("Unable to parse JSON file {:?}: {:?} Skipping...", path, e);
                continue;
            }
            let inclusions_map = json.unwrap().datum_program_inclusions;
            for (pubkey, inc) in inclusions_map.iter() {
                let pk_parsed = pubkey.parse::<Pubkey>().expect(
                    format!(
                        "Bad pubkey provided to datumProgramInclusions in geyser config file {:?}",
                        path
                    )
                    .as_str(),
                );
                datum_program_inclusions.insert(pk_parsed, inc.clone());
            }
        }
    }
    datum_program_inclusions
}
