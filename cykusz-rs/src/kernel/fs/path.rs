pub struct Path<'a>(&'a str);

impl<'a> Path<'a> {
    pub const fn new(path: &'a str) -> Self {
        Path::<'a>(path)
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with("/")
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0
            .trim_start_matches("/")
            .split("/")
            .filter(|e| *e != "" && *e != ".")
    }
}
