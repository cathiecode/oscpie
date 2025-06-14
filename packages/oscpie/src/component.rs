use tiny_skia::Pixmap;

pub trait Component {
    type Props<'a>;
    fn update<'a>(&mut self, _props: &'a Self::Props<'a>) {}
    fn render(&self, pixmap: &mut Pixmap);
}
