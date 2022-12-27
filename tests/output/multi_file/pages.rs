pub mod subpages {
    pub struct Page1 {}
    impl Page1 {
        pub fn render(self) -> String {
            format!("<p>\n    This is subpage 1. I don't do anything special.\n</p>")
        }
        pub fn new() -> Self {
            Self {}
        }
    }
    pub struct Page2 {
        date: String,
    }
    impl Page2 {
        pub fn render(self) -> String {
            let date = self.date;
            format!("<p>\n    I am subpage 2. The current date is {date}.\n</p>")
        }
        pub fn new(date: String) -> Self {
            Self { date }
        }
    }
}
pub struct Index {
    page_name: String,
    cur_time: String,
}
impl Index {
    pub fn render(self) -> String {
        let page_name = self.page_name;
        let cur_time = self.cur_time;
        format!("<p>\n    Welcome to {page_name}. It is currently {cur_time}.\n</p>")
    }
    pub fn new(page_name: String, cur_time: String) -> Self {
        Self { page_name, cur_time }
    }
}
pub struct About {
    author: String,
}
impl About {
    pub fn render(self) -> String {
        let author = self.author;
        format!("<p>\n    This webpage was created by {author}\n</p>")
    }
    pub fn new(author: String) -> Self {
        Self { author }
    }
}
