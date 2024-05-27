use alloc::string::String;

use spin::Once;

pub struct Params {
    map: hashbrown::HashMap<String, String>,
}

impl Params {
    pub fn new() -> Params {
        Params {
            map: hashbrown::HashMap::<String, String>::new(),
        }
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(&String::from(key))
    }

    pub fn put(&mut self, key: &str, value: &str) {
        self.map.insert(String::from(key), String::from(value));
    }
}

static PARAMS: Once<Params> = Once::new();

pub fn init(params: &str) {
    PARAMS.call_once(|| {
        let mut prms = Params::new();
        for param in params.split_ascii_whitespace() {
            if let Some((key, val)) = param.split_once('=') {
                prms.put(key, val);
                logln!("kernel param added {} = {}", key, val);
            }
        }

        prms
    });
}

pub fn params() -> &'static Params {
    PARAMS.get().unwrap()
}

pub fn get(key: &str) -> Option<&String> {
    params().get(key)
}
