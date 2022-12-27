pub struct Basic {
    name: String,
    page_name: String,
}
impl Basic {
    pub fn render(self) -> String {
        let name = self.name;
        let page_name = self.page_name;
        format!("<p>Hello, {name} welcome @{page_name}</p>")
    }
    pub fn new(name: String, page_name: String) -> Self {
        Self { name, page_name }
    }
}
