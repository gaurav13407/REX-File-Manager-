pub enum Pane{
    Left,
    Right,
}

pub struct App{
    pub active_pane:Pane,
    pub should_quit:bool,
}

impl App{
    pub fn new()->Self{
        Self{
            active_pane:Pane::Left,
            should_quit: false,
        }
    }
}
