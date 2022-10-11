use std::io::{Result, Seek};
use std::fs::{File, DirEntry};
use std::path::{PathBuf, Path};
use std::env;

use serde_json::Value;
use rand::seq::SliceRandom;

pub fn cc_asset_dir() -> PathBuf {
    let home = env::var_os("HOME").unwrap();
    let home: &Path = home.as_ref();
    //home.join(".steam/steam/steamapps/common/CrossCode/assets")
    home.join("src/CrossCode/assets/data/maps")
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
	Read,
	Write,
}

fn read_dir(path: &Path, visit: &mut dyn FnMut(&mut Value), top_level: bool, mode: Mode) -> Result<()>{
	let mut options = File::options();
	match mode {
		Mode::Read => options.read(true),
		Mode::Write => options.read(true).write(true),
	};
    for entry in path.read_dir()? {
        match entry {
            Ok(entry) => {
                let mut try_ent = |entry: &DirEntry| -> Result<()> {
                    if entry.file_type()?.is_file() && !top_level {
                        let mut file = options.open(&entry.path())?;
                        let mut map: Value =
                            serde_json::from_reader(&file)?;
                        if let Value::Array(ref mut entities) = map["entities"] {
                        	let visited = entities.iter_mut().fold(false, |v, e| recurse(e, visit) || v);
                        	if mode == Mode::Write && visited {
	                        	file.rewind()?;
	                        	file.set_len(0)?;
	                        	serde_json::to_writer(file, &map)?;
	                        }
                        }

                    } else if entry.file_type()?.is_dir() {
                        read_dir(&entry.path(), visit, false, mode)?;
                    }
                    Ok(())
                };
                if let Err(e) = try_ent(&entry) {
                    eprintln!("Error reading file: {:?}\n\t{}", entry.file_name(), e);
                }
            }
            Err(e) => eprintln!("Error reading file: {}", e),
        }
    }
    Ok(())
}

fn recurse(mut v: &mut Value, visit: &mut dyn FnMut(&mut Value)) -> bool {
	let mut visited = if filter(&v) {
		visit(&mut v);
		true
	} else {
		false
	};
	match v {
		Value::Array(array) => array.iter_mut().fold(visited, |v, e| recurse(e, visit) || v),
		Value::Object(object) => object.values_mut().fold(visited, |v, e| recurse(e, visit) || v),
		_ => visited,
	}
}

fn filter(e: &Value) -> bool {
	match e["item"].as_str() {
		Some("23" | "24" | "25" | "26") => false, // Potential softlock in Rhombus Dungeon
		Some(_) if e["amount"].is_number() => true,
		_ => false
	}
}

fn main() -> Result<()> {
	let mut rng = rand::thread_rng();
	let mut items: Vec<(u32, u64)> = Vec::new();
	let dir = cc_asset_dir();
	read_dir(&dir, &mut |e| {
		let item = e["item"].as_str().unwrap().parse().unwrap();
		let amount = e["amount"].as_u64().unwrap();
		items.push((item, amount));
	}, true, Mode::Read)?;

	items.shuffle(&mut rng);
	
	read_dir(&dir, &mut |e| {
		let (i, a) = items.pop().unwrap();
		e["item"] = i.to_string().into();
		e["amount"] = a.into();
	}, true, Mode::Write)?;

	Ok(())
}