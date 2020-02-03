use core::str::Split;

pub struct Path<'a>(&'a str);

impl<'a> Path<'a> {
    pub const fn new(path: &'a str) -> Self {
        Path::<'a>(path)
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with("/")
    }

    pub fn components(&self) -> Split<'a, &str> {
        self.0.trim_start_matches("/").split("/")
    }
}
