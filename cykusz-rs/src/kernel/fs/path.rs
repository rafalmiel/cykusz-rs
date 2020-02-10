pub struct Path<'a>(&'a str);

impl<'a> Path<'a> {
    pub fn new(path: &'a str) -> Self {
        Path::<'a>(path.trim().trim_end_matches("/"))
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with("/")
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split("/").filter(|e| *e != "" && *e != ".")
    }

    pub fn containing_dir(&self) -> (Path<'a>, Path<'a>) {
        let containing_dir = self.0.rfind("/");

        match containing_dir {
            Some(0) => (Path::new("/"), Path::new(&self.0[1..])),
            Some(v) => (Path::new(&self.0[..v]), Path::new(&self.0[v + 1..])),
            None => (Path::new(""), Path::new(self.0)),
        }
    }

    pub fn str(&self) -> &'a str {
        self.0
    }
}
