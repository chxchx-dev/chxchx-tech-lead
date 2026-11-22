#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
pub mod window;

mod engine;
mod layer;
mod primitive;
mod settings;
mod text;

#[cfg(feature = "image")]
mod raster;

#[cfg(feature = "svg")]
mod vector;

#[cfg(feature = "geometry")]
pub mod geometry;

use iced_debug as debug;
pub use iced_graphics as graphics;
pub use iced_graphics::core;

pub use layer::Layer;
pub use primitive::Primitive;
pub use settings::Settings;

#[cfg(feature = "geometry")]
pub use geometry::Geometry;

use crate::core::renderer;
use crate::core::{
    Background, Color, Font, Pixels, Point, Rectangle, Size, Transformation,
};
use crate::engine::Engine;
use crate::graphics::Viewport;
use crate::graphics::compositor;
use crate::graphics::text::{Editor, Paragraph};

/// A [`tiny-skia`] graphics renderer for [`iced`].
///
/// [`tiny-skia`]: https://github.com/RazrFalcon/tiny-skia
/// [`iced`]: https://github.com/iced-rs/iced
#[derive(Debug)]
pub struct Renderer {
    default_font: Font,
    default_text_size: Pixels,
    layers: layer::Stack,
    engine: Engine, // TODO: Shared engine
}

impl Renderer {
    pub fn new(default_font: Font, default_text_size: Pixels) -> Self {
        Self {
            default_font,
            default_text_size,
            layers: layer::Stack::new(),
            engine: Engine::new(),
        }
    }

    pub fn layers(&mut self) -> &[Layer] {
        self.layers.flush();
        self.layers.as_slice()
    }

    pub fn draw(
        &mut self,
        pixels: &mut tiny_skia::PixmapMut<'_>,
        clip_mask: &mut tiny_skia::Mask,
        viewport: &Viewport,
        damage: &[Rectangle],
        background_color: Color,
    ) {
        let scale_factor = viewport.scale_factor();

        self.layers.flush();

        for &damage_bounds in damage {
            let damage_bounds = damage_bounds * scale_factor;

            let path = tiny_skia::PathBuilder::from_rect(
                tiny_skia::Rect::from_xywh(
                    damage_bounds.x,
                    damage_bounds.y,
                    damage_bounds.width,
                    damage_bounds.height,
                )
                .expect("Create damage rectangle"),
            );

            pixels.fill_path(
                &path,
                &tiny_skia::Paint {
                    shader: tiny_skia::Shader::SolidColor(engine::into_color(
                        background_color,
                    )),
                    anti_alias: false,
                    blend_mode: tiny_skia::BlendMode::Source,
                    ..Default::default()
                },
                tiny_skia::FillRule::default(),
                tiny_skia::Transform::identity(),
                None,
            );

            for layer in self.layers.iter() {
                let Some(layer_bounds) =
                    damage_bounds.intersection(&(layer.bounds * scale_factor))
                else {
                    continue;
                };

                engine::adjust_clip_mask(clip_mask, layer_bounds);

                if !layer.quads.is_empty() {
                    let render_span = debug::render(debug::Primitive::Quad);
                    for (quad, background) in &layer.quads {
                        self.engine.draw_quad(
                            quad,
                            background,
                            Transformation::scale(scale_factor),
                            pixels,
                            clip_mask,
                            layer_bounds,
                        );
                    }
                    render_span.finish();
                }

                if !layer.primitives.is_empty() {
                    let render_span = debug::render(debug::Primitive::Triangle);

                    for group in &layer.primitives {
                        let Some(group_bounds) = (group.clip_bounds()
                            * scale_factor)
                            .intersection(&layer_bounds)
                        else {
                            continue;
                        };

                        engine::adjust_clip_mask(clip_mask, group_bounds);

                        for primitive in group.as_slice() {
                            self.engine.draw_primitive(
                                primitive,
                                Transformation::scale(scale_factor)
                                    * group.transformation(),
                                pixels,
                                clip_mask,
                                group_bounds,
                            );
                        }

                        engine::adjust_clip_mask(clip_mask, layer_bounds);
                    }

                    render_span.finish();
                }

                if !layer.images.is_empty() {
                    let render_span = debug::render(debug::Primitive::Image);

                    for image in &layer.images {
                        self.engine.draw_image(
                            image,
                            Transformation::scale(scale_factor),
                            pixels,
                            clip_mask,
                            layer_bounds,
                        );
                    }

                    render_span.finish();
                }

                if !layer.text.is_empty() {
                    let render_span = debug::render(debug::Primitive::Image);

                    for group in &layer.text {
                        for text in group.as_slice() {
                            self.engine.draw_text(
                                text,
                                Transformation::scale(scale_factor)
                                    * group.transformation(),
                                pixels,
                                clip_mask,
                                layer_bounds,
                            );
                        }
                    }

                    render_span.finish();
                }
            }
        }

        self.engine.trim();
    }
}

impl core::Renderer for Renderer {
    fn start_layer(&mut self, bounds: Rectangle) {
        self.layers.push_clip(bounds);
    }

    fn end_layer(&mut self) {
        self.layers.pop_clip();
    }

    fn start_transformation(&mut self, transformation: Transformation) {
        self.layers.push_transformation(transformation);
    }

    fn end_transformation(&mut self) {
        self.layers.pop_transformation();
    }

    fn fill_quad(
        &mut self,
        quad: renderer::Quad,
        background: impl Into<Background>,
    ) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_quad(quad, background.into(), transformation);
    }

    fn reset(&mut self, new_bounds: Rectangle) {
        self.layers.reset(new_bounds);
    }

    fn allocate_image(
        &mut self,
        _handle: &core::image::Handle,
        callback: impl FnOnce(Result<core::image::Allocation, core::image::Error>)
        + Send
        + 'static,
    ) {
        #[cfg(feature = "image")]
        #[allow(unsafe_code)]
        // TODO: Concurrency
        callback(self.engine.raster_pipeline.load(_handle));

        #[cfg(not(feature = "image"))]
        callback(Err(core::image::Error::Unsupported));
    }
}

impl core::text::Renderer for Renderer {
    type Font = Font;
    type Paragraph = Paragraph;
    type Editor = Editor;

    const ICON_FONT: Font = Font::with_name("Iced-Icons");
    const CHECKMARK_ICON: char = '\u{f00c}';
    const ARROW_DOWN_ICON: char = '\u{e800}';
    const ICED_LOGO: char = '\u{e801}';
    const SCROLL_UP_ICON: char = '\u{e802}';
    const SCROLL_DOWN_ICON: char = '\u{e803}';
    const SCROLL_LEFT_ICON: char = '\u{e804}';
    const SCROLL_RIGHT_ICON: char = '\u{e805}';

    fn default_font(&self) -> Self::Font {
        self.default_font
    }

    fn default_size(&self) -> Pixels {
        self.default_text_size
    }

    fn fill_paragraph(
        &mut self,
        text: &Self::Paragraph,
        position: Point,
        color: Color,
        clip_bounds: Rectangle,
    ) {
        let (layer, transformation) = self.layers.current_mut();

        layer.draw_paragraph(
            text,
            position,
            color,
            clip_bounds,
            transformation,
        );
    }

    fn fill_editor(
        &mut self,
        editor: &Self::Editor,
        position: Point,
        color: Color,
        clip_bounds: Rectangle,
    ) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_editor(editor, position, color, clip_bounds, transformation);
    }

    fn fill_text(
        &mut self,
        text: core::Text,
        position: Point,
        color: Color,
        clip_bounds: Rectangle,
    ) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_text(text, position, color, clip_bounds, transformation);
    }
}

impl graphics::text::Renderer for Renderer {
    fn fill_raw(&mut self, raw: graphics::text::Raw) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_text_raw(raw, transformation);
    }
}

#[cfg(feature = "geometry")]
impl graphics::geometry::Renderer for Renderer {
    type Geometry = Geometry;
    type Frame = geometry::Frame;

    fn new_frame(&self, bounds: Rectangle) -> Self::Frame {
        geometry::Frame::new(bounds)
    }

    fn draw_geometry(&mut self, geometry: Self::Geometry) {
        let (layer, transformation) = self.layers.current_mut();

        match geometry {
            Geometry::Live {
                primitives,
                images,
                text,
                clip_bounds,
            } => {
                layer.draw_primitive_group(
                    primitives,
                    clip_bounds,
                    transformation,
                );

                for image in images {
                    layer.draw_image(image, transformation);
                }

                layer.draw_text_group(text, clip_bounds, transformation);
            }
            Geometry::Cache(cache) => {
                layer.draw_primitive_cache(
                    cache.primitives,
                    cache.clip_bounds,
                    transformation,
                );

                for image in cache.images.iter() {
                    layer.draw_image(image.clone(), transformation);
                }

                layer.draw_text_cache(
                    cache.text,
                    cache.clip_bounds,
                    transformation,
                );
            }
        }
    }
}

impl graphics::mesh::Renderer for Renderer {
    fn draw_mesh(&mut self, _mesh: graphics::Mesh) {
        log::warn!("iced_tiny_skia does not support drawing meshes");
    }

    fn draw_mesh_cache(&mut self, _cache: iced_graphics::mesh::Cache) {
        log::warn!("iced_tiny_skia does not support drawing meshes");
    }
}

#[cfg(feature = "image")]
impl core::image::Renderer for Renderer {
    type Handle = core::image::Handle;

    fn load_image(
        &self,
        handle: &Self::Handle,
    ) -> Result<core::image::Allocation, core::image::Error> {
        self.engine.raster_pipeline.load(handle)
    }

    fn measure_image(
        &self,
        handle: &Self::Handle,
    ) -> Option<crate::core::Size<u32>> {
        self.engine.raster_pipeline.dimensions(handle)
    }

    fn draw_image(
        &mut self,
        image: core::Image,
        bounds: Rectangle,
        clip_bounds: Rectangle,
    ) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_raster(image, bounds, clip_bounds, transformation);
    }
}

#[cfg(feature = "svg")]
impl core::svg::Renderer for Renderer {
    fn measure_svg(
        &self,
        handle: &core::svg::Handle,
    ) -> crate::core::Size<u32> {
        self.engine.vector_pipeline.viewport_dimensions(handle)
    }

    fn draw_svg(
        &mut self,
        svg: core::Svg,
        bounds: Rectangle,
        clip_bounds: Rectangle,
    ) {
        let (layer, transformation) = self.layers.current_mut();
        layer.draw_svg(svg, bounds, clip_bounds, transformation);
    }
}

impl compositor::Default for Renderer {
    type Compositor = window::Compositor;
}

impl renderer::Headless for Renderer {
    async fn new(
        default_font: Font,
        default_text_size: Pixels,
        backend: Option<&str>,
    ) -> Option<Self> {
        if backend.is_some_and(|backend| {
            !["tiny-skia", "tiny_skia"].contains(&backend)
        }) {
            return None;
        }

        Some(Self::new(default_font, default_text_size))
    }

    fn name(&self) -> String {
        "tiny-skia".to_owned()
    }

    fn screenshot(
        &mut self,
        size: Size<u32>,
        scale_factor: f32,
        background_color: Color,
    ) -> Vec<u8> {
        let viewport = Viewport::with_physical_size(size, scale_factor);

        window::compositor::screenshot(self, &viewport, background_color)
    }
}

#[cfg(test)]
mod chxchx_scroll_tests {
    use super::*;
    use crate::core::renderer::{Quad, Renderer as _};
    use crate::core::text::{Paragraph as _, Renderer as _};
    use crate::core::{Border, Shadow, alignment, text};
    use crate::graphics::damage;

    const W: u32 = 400;
    const H: u32 = 300;

    /// One frame of a page that scrolls: a fixed header and a clipped, translated list of cards
    /// with text, like Studio's pages.
    fn frame(renderer: &mut Renderer, offset: f32, rounded: bool, paragraphs: bool) {
        renderer.reset(Rectangle::with_size(Size::new(W as f32, H as f32)));
        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(Point::ORIGIN, Size::new(W as f32, 40.0)),
                border: Border::default(),
                shadow: Shadow::default(),
                snap: false,
            },
            Color::from_rgb(0.1, 0.1, 0.2),
        );
        let view = Rectangle::new(Point::new(0.0, 40.0), Size::new(W as f32, 220.0));
        renderer.start_layer(view);
        renderer.start_transformation(Transformation::translate(0.0, 40.0 - offset));
        for index in 0..40 {
            let top = index as f32 * 56.0;
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(Point::new(8.0, top), Size::new(384.0, 48.0)),
                    border: Border {
                        radius: if rounded { 10.0.into() } else { 0.0.into() },
                        ..Border::default()
                    },
                    shadow: Shadow::default(),
                    snap: false,
                },
                Color::from_rgb(0.2, 0.3, 0.4),
            );
            let spec = text::Text {
        content: format!("Tarjeta número {index} con un texto de ejemplo"),
        bounds: Size::new(360.0, 20.0),
        size: Pixels(14.0),
        line_height: text::LineHeight::default(),
        font: Font::DEFAULT,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::default(),
                };
            let clip = Rectangle::new(Point::ORIGIN, Size::new(W as f32, 100000.0));
            if paragraphs {
                let paragraph = <Renderer as text::Renderer>::Paragraph::with_text(text::Text {
                    content: spec.content.as_str(),
                    bounds: spec.bounds,
                    size: spec.size,
                    line_height: spec.line_height,
                    font: spec.font,
                    align_x: spec.align_x,
                    align_y: spec.align_y,
                    shaping: spec.shaping,
                    wrapping: spec.wrapping,
                });
                renderer.fill_paragraph(&paragraph, Point::new(16.0, top + 14.0), Color::WHITE, clip);
            } else {
                renderer.fill_text(spec, Point::new(16.0, top + 14.0), Color::WHITE, clip);
            }
        }
        renderer.end_transformation();
        renderer.end_layer();
    }

    fn paint(
        renderer: &mut Renderer,
        pixels: &mut [u32],
        mask: &mut tiny_skia::Mask,
        damage: &[Rectangle],
    ) {
        let viewport = Viewport::with_physical_size(Size::new(W, H), 1.0);
        let mut map = tiny_skia::PixmapMut::from_bytes(bytemuck::cast_slice_mut(pixels), W, H)
            .expect("pixel map");
        renderer.draw(&mut map, mask, &viewport, damage, Color::BLACK);
    }

    /// Scrolling with the window's own damage handling must leave exactly what a full redraw would.
    fn scroll_matches_full_redraw(buffers: usize, rounded: bool, paragraphs: bool) {
        let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
        let mut mask = tiny_skia::Mask::new(W, H).unwrap();
        let mut surfaces = vec![vec![0u32; (W * H) as usize]; buffers];
        let mut history: std::collections::VecDeque<Vec<Layer>> = Default::default();
        let full = [Rectangle::with_size(Size::new(W as f32, H as f32))];
        let offsets = [0.0, 3.0, 17.0, 40.0, 41.0, 90.0, 200.0, 199.0, 350.0, 600.0, 640.0, 12.0];
        for (step, offset) in offsets.into_iter().enumerate() {
            frame(&mut renderer, offset, rounded, paragraphs);
            let age = if step < buffers { 0 } else { buffers };
            let last = (age > 0).then(|| history.get(age - 1)).flatten();
            let damage = last
                .map(|last| {
                    damage::diff(last, renderer.layers(), |layer| vec![layer.bounds], Layer::damage)
                })
                .unwrap_or_else(|| full.to_vec());
            if damage.is_empty() {
                if let Some(last) = last {
                    history.push_front(last.clone());
                }
            } else {
                history.push_front(renderer.layers().to_vec());
                history.truncate(buffers);
                let damage = damage::group(damage, full[0]);
                paint(&mut renderer, &mut surfaces[step % buffers], &mut mask, &damage);
            }
            let mut expected = vec![0u32; (W * H) as usize];
            paint(&mut renderer, &mut expected, &mut mask, &full);
            let wrong = surfaces[step % buffers]
                .iter()
                .zip(&expected)
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(wrong, 0, "step {step} (offset {offset}): {wrong} pixels differ from a full redraw");
        }
    }

    #[test]
    fn scrolling_one_buffer() {
        scroll_matches_full_redraw(1, true, false);
    }

    #[test]
    fn scrolling_two_buffers() {
        scroll_matches_full_redraw(2, true, false);
        scroll_matches_full_redraw(2, true, true);
    }

    #[test]
    fn scrolling_three_buffers_square() {
        scroll_matches_full_redraw(3, false, true);
    }
}
